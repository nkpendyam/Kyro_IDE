/**
 * LSP-Monaco Bridge
 *
 * Connects the Tauri LSP backend (Tree-sitter + language servers) to Monaco editor
 * providers for completions, diagnostics, and goto-definition.
 */

import type * as monaco from 'monaco-editor';
import { useKyroStore } from '@/store/kyroStore';

// ── Tauri LSP response types (match Rust structs) ──

interface LspCompletionItem {
  label: string;
  kind: string; // "Function" | "Method" | "Class" | "Struct" | "Variable" | "Keyword" | etc.
  detail: string | null;
  documentation: string | null;
  insert_text: string | null;
}

interface LspDiagnostic {
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
  message: string;
  severity: string; // "error" | "warning" | "information" | "hint"
  code: string | null;
  source: string | null;
}

interface LspLocation {
  uri: string;
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
}

interface LspTextEdit {
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
  new_text: string;
}

interface ScoredCompletionItem {
  label: string;
  kind: string;
  detail: string | null;
  documentation: string | null;
  insert_text: string | null;
  score: number;
  source: string;
}

interface EnhancedCompletionsResponse {
  items: ScoredCompletionItem[];
  total_latency_ms: number;
  sources_used: string[];
  performance_warning: string | null;
}

interface LspServerStatus {
  language: string;
  running: boolean;
}

// ── Helpers ──

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  if (typeof window !== 'undefined' && window.__TAURI__) {
    try {
      return await window.__TAURI__.core.invoke<T>(cmd, args);
    } catch (error) {
      console.warn(`[LSP Bridge] ${cmd} failed:`, error);
      return null;
    }
  }
  return null;
}

const completionKindMap: Record<string, number> = {
  function: 1,
  method: 0,
  class: 5,
  struct: 22,
  interface: 7,
  enum: 15,
  constant: 14,
  variable: 4,
  field: 3,
  keyword: 13,
  snippet: 27,
  text: 18,
  Function: 1,
  Method: 0,
  Class: 5,
  Struct: 22,
  Interface: 7,
  Enum: 15,
  Constant: 14,
  Variable: 4,
  Field: 3,
  Keyword: 13,
  Snippet: 27,
  Text: 18,
};

function toMonacoCompletionKind(kind: string): number {
  return completionKindMap[kind] ?? 18; // default to Text
}

function toMonacoSeverity(
  severity: string,
  monacoModule: typeof monaco
): monaco.MarkerSeverity {
  switch (severity) {
    case 'error':
    case 'Error':
      return monacoModule.MarkerSeverity.Error;
    case 'warning':
    case 'Warning':
      return monacoModule.MarkerSeverity.Warning;
    case 'information':
    case 'Information':
      return monacoModule.MarkerSeverity.Info;
    case 'hint':
    case 'Hint':
      return monacoModule.MarkerSeverity.Hint;
    default:
      return monacoModule.MarkerSeverity.Info;
  }
}

const startedServers = new Set<string>();

function mapLanguageForLspServer(language: string): string {
  const normalized = language.toLowerCase();
  if (['ts', 'tsx', 'js', 'jsx', 'javascript', 'typescript'].includes(normalized)) return 'typescript';
  if (normalized === 'rs' || normalized === 'rust') return 'rust';
  if (normalized === 'py' || normalized === 'python') return 'python';
  if (normalized === 'go') return 'go';
  if (['c', 'h'].includes(normalized)) return 'c';
  if (['cpp', 'cxx', 'cc', 'hpp', 'hxx'].includes(normalized)) return 'cpp';
  return normalized;
}

function toFileUri(monacoModule: typeof monaco, filePath: string): string {
  if (!filePath) return '';
  if (filePath.startsWith('file://')) return filePath;
  return monacoModule.Uri.file(filePath).toString();
}

function getRootUri(monacoModule: typeof monaco, filePath: string): string {
  if (!filePath) return '';
  const normalized = filePath.replace(/\\/g, '/');
  const lastSlash = normalized.lastIndexOf('/');
  const rootPath = lastSlash > 0 ? normalized.slice(0, lastSlash) : normalized;
  return toFileUri(monacoModule, rootPath);
}

async function ensureServerStarted(monacoModule: typeof monaco, language: string, filePath: string): Promise<void> {
  const mappedLanguage = mapLanguageForLspServer(language);
  if (!mappedLanguage || startedServers.has(mappedLanguage)) return;

  const rootUri = getRootUri(monacoModule, filePath);
  if (!rootUri) return;

  const status = await invokeTauri<LspServerStatus>('lsp_start_server', {
    language: mappedLanguage,
    rootUri,
  });

  if (status?.running) {
    startedServers.add(mappedLanguage);
  }
}

export const __testing = {
  mapLanguageForLspServer,
  toFileUri,
  getRootUri,
  toMonacoSeverity,
};

// ── Registration Functions ──

/**
 * Register all LSP providers on a Monaco editor instance.
 * Call this once in handleEditorMount.
 */
export function registerLspProviders(
  monacoModule: typeof monaco,
  editor: monaco.editor.IStandaloneCodeEditor,
  getFilePath: () => string,
  getLanguage: () => string
): () => void {
  let diagnosticTimer: ReturnType<typeof setTimeout> | null = null;
  const disposables: monaco.IDisposable[] = [];
  // 1. Completion provider (all languages)
  const completionDisposable = monacoModule.languages.registerCompletionItemProvider(
    { pattern: '**' },
    {
      triggerCharacters: ['.', ':', '<', '"', "'", '/', '@', '#'],
      provideCompletionItems: async (model, position) => {
        const code = model.getValue();
        const language = getLanguage();
        const filePath = getFilePath();
        const uri = toFileUri(monacoModule, filePath);
        const word = model.getWordUntilPosition(position);

        await ensureServerStarted(monacoModule, language, filePath);

        // Try AI-enhanced completions first
        const enhanced = await invokeTauri<EnhancedCompletionsResponse>(
          'get_ai_completions',
          {
            request: {
              file_path: filePath,
              language,
              code,
              line: position.lineNumber - 1, // Rust uses 0-indexed
              column: position.column - 1,
              trigger_kind: 'invoked',
              prefix: word.word,
            },
          }
        );

        if (enhanced && enhanced.items.length > 0) {
          return {
            suggestions: enhanced.items.map((item, i) => ({
              label: item.label,
              kind: toMonacoCompletionKind(item.kind) as monaco.languages.CompletionItemKind,
              detail: item.detail || `${item.source} (${(item.score * 100).toFixed(0)}%)`,
              documentation: item.documentation || undefined,
              insertText: item.insert_text || item.label,
              range: {
                startLineNumber: position.lineNumber,
                startColumn: word.startColumn,
                endLineNumber: position.lineNumber,
                endColumn: word.endColumn,
              },
              sortText: String(i).padStart(4, '0'),
            })),
          };
        }

        // Fallback to basic Tree-sitter completions
        const result = await invokeTauri<LspCompletionItem[]>(
          'lsp_get_completions',
          {
            uri,
            line: position.lineNumber - 1,
            character: position.column - 1,
          }
        );

        if (!result || result.length === 0) {
          return { suggestions: [] };
        }

        return {
          suggestions: result.map((item, i) => ({
            label: item.label,
            kind: toMonacoCompletionKind(item.kind) as monaco.languages.CompletionItemKind,
            detail: item.detail || undefined,
            documentation: item.documentation || undefined,
            insertText: item.insert_text || item.label,
            range: {
              startLineNumber: position.lineNumber,
              startColumn: word.startColumn,
              endLineNumber: position.lineNumber,
              endColumn: word.endColumn,
            },
            sortText: String(i).padStart(4, '0'),
          })),
        };
      },
    }
  );
  disposables.push(completionDisposable);

  // 2. Definition provider (goto-definition via F12)
  const definitionDisposable = monacoModule.languages.registerDefinitionProvider(
    { pattern: '**' },
    {
      provideDefinition: async (model, position) => {
        const filePath = getFilePath();
        const language = getLanguage();
        const uri = toFileUri(monacoModule, filePath);

        await ensureServerStarted(monacoModule, language, filePath);

        const result = await invokeTauri<LspLocation | null>(
          'lsp_goto_definition',
          {
            uri,
            line: position.lineNumber - 1,
            character: position.column - 1,
          }
        );

        if (!result) return null;

        return {
          uri: monacoModule.Uri.parse(result.uri),
          range: {
            startLineNumber: result.range.start.line + 1,
            startColumn: result.range.start.character + 1,
            endLineNumber: result.range.end.line + 1,
            endColumn: result.range.end.character + 1,
          },
        } as monaco.languages.Location;
      },
    }
  );
  disposables.push(definitionDisposable);

  // 3. Hover provider
  const hoverDisposable = monacoModule.languages.registerHoverProvider(
    { pattern: '**' },
    {
      provideHover: async (model, position) => {
        const filePath = getFilePath();
        const language = getLanguage();
        const uri = toFileUri(monacoModule, filePath);

        await ensureServerStarted(monacoModule, language, filePath);

        const result = await invokeTauri<{ contents: string; range?: LspLocation['range'] } | null>(
          'lsp_hover',
          {
            uri,
            line: position.lineNumber - 1,
            character: position.column - 1,
          }
        );

        if (!result?.contents) return null;

        return {
          contents: [{ value: result.contents }],
        };
      },
    }
  );
  disposables.push(hoverDisposable);

  // 4. Diagnostics — poll on content change with debounce
  const runDiagnostics = async () => {
    const model = editor.getModel();
    if (!model) return;

    const filePath = getFilePath();
    const uri = toFileUri(monacoModule, filePath);
    const language = getLanguage();

    await ensureServerStarted(monacoModule, language, filePath);

    const result = await invokeTauri<LspDiagnostic[]>(
      'lsp_get_diagnostics',
      { uri }
    );

    if (!result) return;

    const markers: monaco.editor.IMarkerData[] = result.map((d) => ({
      severity: toMonacoSeverity(d.severity, monacoModule),
      message: d.message,
      startLineNumber: d.range.start.line + 1,
      startColumn: d.range.start.character + 1,
      endLineNumber: d.range.end.line + 1,
      endColumn: d.range.end.character + 1,
      code: d.code || undefined,
      source: d.source || 'kyro-lsp',
    }));

    monacoModule.editor.setModelMarkers(model, 'kyro-lsp', markers);

    // Update diagnostic counts in store
    const errors = markers.filter((m) => m.severity === monacoModule.MarkerSeverity.Error).length;
    const warnings = markers.filter((m) => m.severity === monacoModule.MarkerSeverity.Warning).length;
    useKyroStore.getState().setDiagnosticCounts(errors, warnings);
  };

  const contentChangeDisposable = editor.onDidChangeModelContent(() => {
    if (diagnosticTimer) clearTimeout(diagnosticTimer);
    diagnosticTimer = setTimeout(runDiagnostics, 500); // debounce 500ms
  });
  disposables.push(contentChangeDisposable);

  // Run diagnostics on initial mount
  runDiagnostics();

  // 5. Format-on-save action
  const formatAction = editor.addAction({
    id: 'kyro.formatDocument',
    label: 'Format Document (Kyro LSP)',
    keybindings: [
      monacoModule.KeyMod.Alt | monacoModule.KeyMod.Shift | monacoModule.KeyCode.KeyF,
    ],
    run: async (ed) => {
      const model = ed.getModel();
      if (!model) return;

      const uri = getFilePath();
      const edits = await invokeTauri<LspTextEdit[]>('lsp_format_document', { uri });

      if (!edits || edits.length === 0) return;

      // Apply edits in reverse order to preserve positions
      const monacoEdits = edits
        .slice()
        .reverse()
        .map((e) => ({
          range: new monacoModule.Range(
            e.range.start.line + 1,
            e.range.start.character + 1,
            e.range.end.line + 1,
            e.range.end.character + 1
          ),
          text: e.new_text,
        }));

      model.pushEditOperations([], monacoEdits, () => null);
    },
  });
  disposables.push(formatAction);

  // 6. Update symbol table on save (for better completions over time)
  const saveDisposable = editor.addCommand(
    monacoModule.KeyMod.CtrlCmd | monacoModule.KeyCode.KeyS,
    async () => {
      const model = editor.getModel();
      if (!model) return;

      // Update symbol table for AI completions
      invokeTauri('update_file_symbols', {
        file_path: getFilePath(),
        code: model.getValue(),
        language: getLanguage(),
      });
    }
  );
  // saveDisposable is string | null for addCommand, not IDisposable

  // Return cleanup function
  return () => {
    if (diagnosticTimer) clearTimeout(diagnosticTimer);
    disposables.forEach((d) => d.dispose());
    disposables.length = 0;
  };
}

/**
 * Setup file watcher via Tauri and call onChanged when files change.
 * Returns a cleanup function.
 */
export async function setupFileWatcher(
  directoryPath: string,
  onChanged: () => void
): Promise<() => void> {
  // Start watching
  await invokeTauri('watch_directory', { path: directoryPath });

  // Listen for file-changed events
  let listenUnlisten: (() => void) | null = null;

  if (typeof window !== 'undefined' && window.__TAURI__) {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      listenUnlisten = await listen('file-changed', () => {
        onChanged();
      }) as unknown as () => void;
    } catch {
      // Event API not available
    }
  }

  return () => {
    invokeTauri('unwatch_directory', { path: directoryPath });
    listenUnlisten?.();
  };
}
