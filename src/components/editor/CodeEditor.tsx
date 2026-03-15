'use client';

import React, { useRef, useState, useCallback, useEffect } from 'react';
import type * as monaco from 'monaco-editor';
import Editor, { Monaco } from '@monaco-editor/react';
import { useKyroStore } from '@/store/kyroStore';
import { useTheme } from '@/components/theme/ThemeProvider';
import { registerLspProviders } from '@/lib/lspBridge';
import { applyMonacoTheme } from '@/lib/themeSystem';
import { useGhostTextProvider, GhostTextOverlay, InlineChatWidget } from './GhostTextProvider';
import { MinimapToggle, useEditorMinimap } from './Minimap';
import { EditorPresence } from '@/components/collaboration/EditorPresence';

export interface CodeEditorProps {
  onSave?: () => void;
  roomId?: string;
  currentUserId?: string;
  currentUserName?: string;
}

export function CodeEditor({ onSave, roomId, currentUserId, currentUserName }: CodeEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<typeof import('monaco-editor') | null>(null);
  const lspCleanupRef = useRef<(() => void) | null>(null);
  const [editorInstance, setEditorInstance] = useState<monaco.editor.IStandaloneCodeEditor | null>(null);

  const [splitEditor, setSplitEditor] = useState<'none' | 'horizontal' | 'vertical'>('none');
  const [inlineChatOpen, setInlineChatOpen] = useState(false);
  const [inlineChatPosition, setInlineChatPosition] = useState<monaco.Position | null>(null);
  const [inlineChatSelection, setInlineChatSelection] = useState<monaco.Selection | null>(null);

  const {
    openFiles,
    activeFileIndex,
    updateFileContent,
    setCursorPosition,
    settings,
    minimapVisible,
    setMinimapVisible,
    setEditorOptions,
    createEditorGroup,
    addTabToGroup,
    splitDirection,
    setSplitDirection,
  } = useKyroStore();

  const { theme } = useTheme();

  const currentFile = activeFileIndex >= 0 ? openFiles[activeFileIndex] : null;

  const { currentCompletion } = useGhostTextProvider(
    editorRef,
    monacoRef,
    currentFile?.language || 'plaintext',
    {
      enabled: settings.editorOptions.inlineSuggest,
      debounceMs: 300,
      maxTokens: 100,
    }
  );

  useEditorMinimap(editorInstance);

  const setupEditorCommands = useCallback((editor: monaco.editor.IStandaloneCodeEditor, monacoModule: Monaco) => {
    editor.addCommand(monacoModule.KeyMod.CtrlCmd | monacoModule.KeyCode.KeyS, () => {
      onSave?.();
    });

    editor.addCommand(monacoModule.KeyMod.CtrlCmd | monacoModule.KeyCode.KeyK, () => {
      const selection = editor.getSelection();
      const position = editor.getPosition();
      setInlineChatSelection(selection);
      setInlineChatPosition(position);
      setInlineChatOpen(true);
    });

    editor.addCommand(monacoModule.KeyMod.CtrlCmd | monacoModule.KeyCode.Backslash, () => {
      if (splitDirection !== 'none' || !currentFile) return;
      const newGroupId = createEditorGroup();
      addTabToGroup(newGroupId, {
        path: currentFile.path,
        content: currentFile.content,
        language: currentFile.language,
        isDirty: currentFile.isDirty,
      });
      setSplitDirection('vertical');
      setSplitEditor('vertical');
    });

    editor.addCommand(monacoModule.KeyMod.CtrlCmd | monacoModule.KeyMod.Shift | monacoModule.KeyCode.Backslash, () => {
      if (splitDirection !== 'none' || !currentFile) return;
      const newGroupId = createEditorGroup();
      addTabToGroup(newGroupId, {
        path: currentFile.path,
        content: currentFile.content,
        language: currentFile.language,
        isDirty: currentFile.isDirty,
      });
      setSplitDirection('horizontal');
      setSplitEditor('horizontal');
    });
  }, [onSave, splitDirection, currentFile, createEditorGroup, addTabToGroup, setSplitDirection]);

  const handleEditorMount = useCallback((editor: monaco.editor.IStandaloneCodeEditor, monacoModule: Monaco) => {
    editorRef.current = editor;
    monacoRef.current = monacoModule;
    setEditorInstance(editor);

    applyMonacoTheme(monacoModule, theme);

    editor.onDidChangeCursorPosition((e) => {
      setCursorPosition(e.position.lineNumber, e.position.column);
      window.dispatchEvent(new CustomEvent('kyro:cursor-change', {
        detail: { line: e.position.lineNumber, column: e.position.column },
      }));
    });

    setupEditorCommands(editor, monacoModule);

    if (lspCleanupRef.current) {
      lspCleanupRef.current();
    }

    lspCleanupRef.current = registerLspProviders(
      monacoModule,
      editor,
      () => {
        const state = useKyroStore.getState();
        const file = state.activeFileIndex >= 0 ? state.openFiles[state.activeFileIndex] : null;
        return file?.path || '';
      },
      () => {
        const state = useKyroStore.getState();
        const file = state.activeFileIndex >= 0 ? state.openFiles[state.activeFileIndex] : null;
        return file?.language || 'plaintext';
      }
    );

    editor.focus();
  }, [setCursorPosition, setupEditorCommands, theme]);

  useEffect(() => {
    if (!monacoRef.current) return;
    applyMonacoTheme(monacoRef.current, theme);
  }, [theme]);

  useEffect(() => {
    return () => {
      lspCleanupRef.current?.();
    };
  }, []);

  const handleEditorChange = useCallback((value: string | undefined) => {
    if (value !== undefined && currentFile) {
      updateFileContent(currentFile.path, value);
    }
  }, [currentFile, updateFileContent]);

  if (!currentFile) {
    return (
      <div data-testid="editor-container" className="h-full flex flex-col items-center justify-center text-[#8b949e]">
        <p className="text-lg mb-2">No file open</p>
        <p className="text-xs">Select a file from explorer or press Ctrl+P</p>
      </div>
    );
  }

  const editorOptions: monaco.editor.IStandaloneEditorConstructionOptions = {
    fontSize: settings.editorOptions.fontSize,
    fontFamily: settings.editorOptions.fontFamily,
    minimap: { enabled: minimapVisible, showSlider: 'mouseover' },
    scrollBeyondLastLine: false,
    wordWrap: settings.editorOptions.wordWrap,
    automaticLayout: true,
    tabSize: settings.editorOptions.tabSize,
    insertSpaces: true,
    lineNumbers: settings.editorOptions.lineNumbers,
    renderWhitespace: settings.editorOptions.renderWhitespace,
    bracketPairColorization: { enabled: settings.editorOptions.bracketPairColorization },
    guides: {
      bracketPairs: settings.editorOptions.bracketPairColorization,
      indentation: true,
    },
    stickyScroll: { enabled: settings.editorOptions.stickyScroll },
    inlineSuggest: { enabled: settings.editorOptions.inlineSuggest },
    quickSuggestions: { other: true, comments: false, strings: true },
    suggestOnTriggerCharacters: true,
    parameterHints: { enabled: true },
    formatOnPaste: true,
    formatOnType: true,
    folding: true,
    renderLineHighlight: 'all',
    cursorBlinking: 'smooth',
    cursorSmoothCaretAnimation: 'on',
    smoothScrolling: true,
    mouseWheelZoom: true,
    links: true,
    colorDecorators: true,
    padding: { top: 16, bottom: 16 },
  };

  return (
    <div data-testid="editor-container" className="h-full flex flex-col overflow-hidden" ref={containerRef}>
      <div className="h-8 bg-[#161b22] border-b border-[#30363d] flex items-center px-2 text-xs text-[#8b949e]">
        <MinimapToggle visible={minimapVisible} onToggle={() => setMinimapVisible(!minimapVisible)} />
        <button
          onClick={() => {
            const next = settings.editorOptions.wordWrap === 'on' ? 'off' : 'on';
            setEditorOptions({ wordWrap: next });
          }}
          className={`ml-2 px-2 py-1 rounded hover:bg-[#21262d] ${settings.editorOptions.wordWrap === 'on' ? 'text-[#58a6ff]' : ''}`}
        >
          Wrap
        </button>
      </div>

      <div className={`flex-1 flex ${splitEditor === 'vertical' ? 'flex-row' : 'flex-col'} overflow-hidden relative`}>
        <div className={`flex-1 overflow-hidden ${splitEditor !== 'none' ? 'border-r border-[#30363d]' : ''}`}>
          <Editor
            height="100%"
            language={currentFile.language}
            value={currentFile.content}
            onChange={handleEditorChange}
            onMount={handleEditorMount}
            options={editorOptions}
          />
        </div>

        {splitEditor !== 'none' && (
          <div className="flex-1 overflow-hidden">
            <Editor
              height="100%"
              language={currentFile.language}
              value={currentFile.content}
              onChange={handleEditorChange}
              onMount={(secondEditor, monacoModule) => {
                applyMonacoTheme(monacoModule, theme);
                secondEditor.updateOptions({ minimap: { enabled: false } });
              }}
              options={{ ...editorOptions, minimap: { enabled: false } }}
            />
          </div>
        )}

        <GhostTextOverlay editor={editorInstance} completion={currentCompletion} />

        {inlineChatOpen && (
          <InlineChatWidget
            editor={editorInstance}
            position={inlineChatPosition}
            selection={inlineChatSelection}
            onClose={() => {
              setInlineChatOpen(false);
              setInlineChatPosition(null);
              setInlineChatSelection(null);
            }}
          />
        )}

        {inlineChatOpen && (
          <div className="absolute bottom-4 right-4 text-xs text-[#8b949e] pointer-events-none">
            Inline chat active
          </div>
        )}

        <EditorPresence
          editorContainerRef={containerRef as React.RefObject<HTMLDivElement>}
          roomId={roomId}
          currentUserId={currentUserId}
          currentUserName={currentUserName}
        />
      </div>
    </div>
  );
}
