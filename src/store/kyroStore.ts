import { create } from 'zustand';

export interface FileNode { name: string; path: string; is_directory: boolean; children?: FileNode[]; extension?: string; size?: number; }
export interface OpenFile { path: string; content: string; language: string; isDirty: boolean; isPinned?: boolean; isPreview?: boolean; }
export interface ChatMessage { id: string; role: 'user' | 'assistant'; content: string; timestamp: Date; isStreaming?: boolean; ragSources?: RagSource[]; }

// RAG Source for context
export interface RagSource {
  file_path: string;
  start_line: number;
  end_line: number;
  score: number;
  preview: string;
}

export interface ReviewChecklistItem {
  label: string;
  checked: boolean;
  detail?: string | null;
}

export interface ReviewComment {
  id: string;
  severity: 'info' | 'warning' | 'error' | string;
  title: string;
  body: string;
  line?: number | null;
  suggestion?: string | null;
}

export interface DiffReviewResult {
  summary: string;
  risk: 'low' | 'medium' | 'high' | string;
  checklist: ReviewChecklistItem[];
  comments: ReviewComment[];
}

// Editor Group for split panes
export interface EditorTab {
  path: string;
  content: string;
  language: string;
  isDirty: boolean;
  isPinned?: boolean;
  viewState?: unknown; // Monaco editor view state
}

export interface EditorGroup {
  id: string;
  tabs: EditorTab[];
  activeTabIndex: number;
  width?: number; // percentage or pixels
  height?: number;
}

export type SplitDirection = 'none' | 'horizontal' | 'vertical';

// Multi-cursor history for undo operations
export interface CursorOperation {
  type: 'add' | 'remove' | 'move';
  selections: Array<{ startLine: number; startCol: number; endLine: number; endCol: number }>;
}
export interface ModelInfo { name: string; size: string; modified_at: string; }

// Embedded LLM types
export interface HardwareInfo {
  gpu_name: string | null;
  vram_gb: number;
  ram_gb: number;
  cpu_cores: number;
  backend: string;
  memory_tier: string;
  recommended_model: string;
}

export interface LocalModelInfo {
  name: string;
  size_mb: number;
  downloaded: boolean;
  loaded: boolean;
  quantization: string;
  min_memory_tier: string;
}

// Ghost text state
export interface GhostTextCompletion {
  id: string;
  text: string;
  position: { lineNumber: number; column: number };
  isStreaming: boolean;
  timestamp: number;
  latency: number; // Time to first token in ms
}

export interface GhostTextCacheStats {
  hits: number;
  misses: number;
  size: number;
  hitRate: number;
}

export interface GhostTextConfig {
  enabled: boolean;
  debounceMs: number;
  maxTokens: number;
  temperature: number;
  triggerOnTyping: boolean;
  triggerOnNewline: boolean;
  minPrefixLength: number;
  showAcceptHint: boolean;
  cacheEnabled: boolean;
  cacheMaxSize: number;
}

export interface GitStatus { branch: string; ahead: number; behind: number; staged: Array<{ path: string; status: string }>; unstaged: Array<{ path: string; status: string }>; untracked: string[]; }

// Editor settings
export interface EditorSettings {
  fontSize: number;
  fontFamily: string;
  tabSize: number;
  wordWrap: 'on' | 'off' | 'bounded';
  minimap: boolean;
  lineNumbers: 'on' | 'off' | 'relative';
  renderWhitespace: 'none' | 'boundary' | 'selection' | 'all';
  bracketPairColorization: boolean;
  stickyScroll: boolean;
  inlineSuggest: boolean;
  formatOnSave: boolean;
  autoSave: 'off' | 'afterDelay' | 'onFocusChange';
}

// Scope info for breadcrumbs
export interface ScopeInfo {
  name: string;
  kind: 'function' | 'class' | 'method' | 'interface' | 'struct' | 'enum';
  line: number;
}

// Search result
export interface SearchResult {
  file: string;
  line: number;
  column: number;
  text: string;
  context: string;
}

// Debug state
export interface DebugBreakpoint {
  id: string;
  file: string;
  line: number;
  enabled: boolean;
  condition?: string;
}

interface KyroState {
  projectPath: string | null; fileTree: FileNode | null; openFiles: OpenFile[]; activeFileIndex: number;
  editorContent: string; cursorPosition: { line: number; column: number }; selectedText: string;
  diagnosticCounts: { errors: number; warnings: number };
  isOllamaRunning: boolean; models: ModelInfo[]; selectedModel: string; chatMessages: ChatMessage[]; isAiLoading: boolean;
  terminalOutput: string; gitStatus: GitStatus | null; supportedLanguages: string[];
  sidebarWidth: number; chatWidth: number; showChat: boolean; showTerminal: boolean; terminalHeight: number;
  
  // Embedded LLM state
  hardwareInfo: HardwareInfo | null;
  localModels: LocalModelInfo[];
  isEmbeddedLLMReady: boolean;
  isEmbeddedLLMLoading: boolean;
  selectedLocalModel: string;
  inferenceStats: { totalTokens: number; totalTime: number; avgTokensPerSecond: number };
  
  // New state
  settings: { editorOptions: EditorSettings; theme: string; keybindings: string; };
  currentScope: ScopeInfo | null;
  searchResults: SearchResult[];
  isSearching: boolean;
  breakpoints: DebugBreakpoint[];
  commandPaletteOpen: boolean;
  symbolSearchOpen: boolean;
  globalSearchOpen: boolean;
  inlineChatOpen: boolean;
  inlineChatPosition: { x: number; y: number } | null;
  recentCommands: string[];
  recentFiles: string[];
  
  // Editor Groups / Split Panes
  editorGroups: EditorGroup[];
  activeGroupId: string;
  splitDirection: SplitDirection;
  draggedTab: { groupId: string; tabIndex: number } | null;
  
  // Multi-cursor history
  cursorHistory: CursorOperation[];
  maxCursorHistory: number;
  
  // Minimap state
  minimapVisible: boolean;
  minimapScale: number;
  
  // Ghost text state
  ghostTextCompletion: GhostTextCompletion | null;

  // Autopilot state
  autopilotMode: 'default' | 'yolo' | 'autopilot';
  isAgentRunning: boolean;
  
  // Checkpoints state
  checkpoints: Array<{
    id: string;
    label: string;
    timestamp: number;
    messageIndex: number;
    fileSnapshots: Record<string, string>;
    description: string;
    isAutomatic: boolean;
  }>;
  
  // Project rules
  projectRules: Array<{
    id: string;
    name: string;
    content: string;
    enabled: boolean;
    source: string;
  }>;
  ghostTextCacheStats: GhostTextCacheStats;
  ghostTextConfig: GhostTextConfig;
  isGhostTextProcessing: boolean;
  
  setProjectPath: (path: string | null) => void; setFileTree: (tree: FileNode | null) => void;
  openFile: (file: OpenFile) => void; closeFile: (path: string) => void; closeAllFiles: () => void; closeOtherFiles: (path: string) => void; setActiveFile: (index: number) => void; updateFileContent: (path: string, content: string) => void;
  setEditorContent: (content: string) => void; setCursorPosition: (line: number, column: number) => void; setSelectedText: (text: string) => void;
  setDiagnosticCounts: (errors: number, warnings: number) => void;
  setOllamaStatus: (running: boolean) => void; setModels: (models: ModelInfo[]) => void; setSelectedModel: (model: string) => void;
  addChatMessage: (message: ChatMessage) => void; upsertChatMessage: (message: ChatMessage) => void; clearChatMessages: () => void; setAiLoading: (loading: boolean) => void;
  setTerminalOutput: (output: string) => void; appendTerminalOutput: (output: string) => void; setGitStatus: (status: GitStatus | null) => void;
  setSupportedLanguages: (languages: string[]) => void;
  setSidebarWidth: (width: number) => void; setChatWidth: (width: number) => void; toggleChat: () => void; toggleTerminal: () => void; setTerminalHeight: (height: number) => void;
  setShowTerminal: (show: boolean) => void; setShowChat: (show: boolean) => void;
  
  // Embedded LLM actions
  setHardwareInfo: (info: HardwareInfo) => void;
  setLocalModels: (models: LocalModelInfo[]) => void;
  setEmbeddedLLMReady: (ready: boolean) => void;
  setEmbeddedLLMLoading: (loading: boolean) => void;
  setSelectedLocalModel: (model: string) => void;
  updateInferenceStats: (tokens: number, timeMs: number) => void;
  
  // New actions
  setEditorOptions: (options: Partial<EditorSettings>) => void;
  setTheme: (theme: string) => void;
  setKeybindings: (scheme: string) => void;
  setCurrentScope: (scope: ScopeInfo | null) => void;
  setSearchResults: (results: SearchResult[]) => void;
  setIsSearching: (searching: boolean) => void;
  addBreakpoint: (breakpoint: DebugBreakpoint) => void;
  removeBreakpoint: (id: string) => void;
  toggleBreakpoint: (id: string) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setSymbolSearchOpen: (open: boolean) => void;
  setGlobalSearchOpen: (open: boolean) => void;
  setInlineChatOpen: (open: boolean, position?: { x: number; y: number } | null) => void;
  addRecentCommand: (command: string) => void;
  addRecentFile: (path: string) => void;
  pinFile: (path: string) => void;
  unpinFile: (path: string) => void;
  
  // Ghost text actions
  setGhostTextCompletion: (completion: GhostTextCompletion | null) => void;
  updateGhostTextCompletion: (text: string) => void;
  setGhostTextProcessing: (processing: boolean) => void;
  updateGhostTextCacheStats: (stats: Partial<GhostTextCacheStats>) => void;
  setGhostTextConfig: (config: Partial<GhostTextConfig>) => void;
  clearGhostText: () => void;
  
  // Editor Groups / Split Panes actions
  createEditorGroup: (id?: string) => string;
  closeEditorGroup: (id: string) => void;
  setActiveGroup: (id: string) => void;
  addTabToGroup: (groupId: string, tab: EditorTab) => void;
  removeTabFromGroup: (groupId: string, tabIndex: number) => void;
  setActiveTabInGroup: (groupId: string, tabIndex: number) => void;
  moveTabBetweenGroups: (fromGroupId: string, fromIndex: number, toGroupId: string, toIndex: number) => void;
  setSplitDirection: (direction: SplitDirection) => void;
  setDraggedTab: (dragInfo: { groupId: string; tabIndex: number } | null) => void;
  updateGroupSize: (groupId: string, width?: number, height?: number) => void;
  saveEditorViewState: (groupId: string, viewState: unknown) => void;
  
  // Multi-cursor history actions
  pushCursorOperation: (operation: CursorOperation) => void;
  popCursorOperation: () => CursorOperation | undefined;
  clearCursorHistory: () => void;
  
  // Minimap actions
  setMinimapVisible: (visible: boolean) => void;
  setMinimapScale: (scale: number) => void;
  
  // Autopilot actions
  setAutopilotMode: (mode: 'default' | 'yolo' | 'autopilot') => void;
  setAgentRunning: (running: boolean) => void;
  
  // Checkpoint actions
  addCheckpoint: (checkpoint: KyroState['checkpoints'][0]) => void;
  removeCheckpoint: (id: string) => void;
  clearCheckpoints: () => void;
  
  // Project rules actions
  setProjectRules: (rules: KyroState['projectRules']) => void;
  updateProjectRule: (id: string, updates: Partial<KyroState['projectRules'][0]>) => void;
}

export const useKyroStore = create<KyroState>((set, get) => ({
  projectPath: null, fileTree: null, openFiles: [], activeFileIndex: -1, editorContent: '', cursorPosition: { line: 1, column: 1 }, selectedText: '',
  diagnosticCounts: { errors: 0, warnings: 0 },
  isOllamaRunning: false, models: [], selectedModel: 'codellama:7b', chatMessages: [], isAiLoading: false, terminalOutput: '', gitStatus: null, supportedLanguages: [],
  sidebarWidth: 260, chatWidth: 400, showChat: true, showTerminal: true, terminalHeight: 200,
  
  // Embedded LLM initial state
  hardwareInfo: null,
  localModels: [],
  isEmbeddedLLMReady: false,
  isEmbeddedLLMLoading: false,
  selectedLocalModel: 'qwen3-4b-q4_k_m',
  inferenceStats: { totalTokens: 0, totalTime: 0, avgTokensPerSecond: 0 },
  
  // New state defaults
  settings: {
    editorOptions: {
      fontSize: 14,
      fontFamily: 'JetBrains Mono, Fira Code, monospace',
      tabSize: 4,
      wordWrap: 'on',
      minimap: true,
      lineNumbers: 'on',
      renderWhitespace: 'selection',
      bracketPairColorization: true,
      stickyScroll: true,
      inlineSuggest: true,
      formatOnSave: true,
      autoSave: 'afterDelay'
    },
    theme: 'kro-dark',
    keybindings: 'vscode'
  },
  currentScope: null,
  searchResults: [],
  isSearching: false,
  breakpoints: [],
  commandPaletteOpen: false,
  symbolSearchOpen: false,
  globalSearchOpen: false,
  inlineChatOpen: false,
  inlineChatPosition: null,
  recentCommands: [],
  recentFiles: [],
  
  // Ghost text initial state
  ghostTextCompletion: null,
  ghostTextCacheStats: { hits: 0, misses: 0, size: 0, hitRate: 0 },
  ghostTextConfig: {
    enabled: true,
    debounceMs: 200,
    maxTokens: 100,
    temperature: 0.3,
    triggerOnTyping: true,
    triggerOnNewline: true,
    minPrefixLength: 3,
    showAcceptHint: true,
    cacheEnabled: true,
    cacheMaxSize: 1000,
  },
  isGhostTextProcessing: false,
  
  // Editor Groups / Split Panes initial state
  editorGroups: [{ id: 'main', tabs: [], activeTabIndex: -1 }],
  activeGroupId: 'main',
  splitDirection: 'none',
  draggedTab: null,
  
  // Multi-cursor history initial state
  cursorHistory: [],
  maxCursorHistory: 50,
  
  // Minimap initial state
  minimapVisible: true,
  minimapScale: 1,
  
  // Autopilot initial state
  autopilotMode: 'default',
  isAgentRunning: false,
  
  // Checkpoints initial state
  checkpoints: [],
  
  // Project rules initial state
  projectRules: [],
  
  setProjectPath: (path) => set({ projectPath: path }), setFileTree: (tree) => set({ fileTree: tree }),
  openFile: (file) => {
    const { openFiles } = get();
    const existingIndex = openFiles.findIndex(f => f.path === file.path);
    if (existingIndex >= 0) { set({ activeFileIndex: existingIndex, editorContent: file.content }); }
    else { set({ openFiles: [...openFiles, file], activeFileIndex: openFiles.length, editorContent: file.content }); }
  },
  closeFile: (path) => {
    const { openFiles, activeFileIndex } = get();
    const newFiles = openFiles.filter(f => f.path !== path);
    let newIndex = activeFileIndex;
    if (newFiles.length === 0) newIndex = -1;
    else if (activeFileIndex >= newFiles.length) newIndex = newFiles.length - 1;
    set({ openFiles: newFiles, activeFileIndex: newIndex, editorContent: newIndex >= 0 ? newFiles[newIndex].content : '' });
  },
  closeAllFiles: () => {
    set({ openFiles: [], activeFileIndex: -1, editorContent: '' });
  },
  closeOtherFiles: (path) => {
    const { openFiles } = get();
    const kept = openFiles.filter(f => f.path === path);
    set({ openFiles: kept, activeFileIndex: kept.length > 0 ? 0 : -1, editorContent: kept.length > 0 ? kept[0].content : '' });
  },
  setActiveFile: (index) => { const { openFiles } = get(); if (index >= 0 && index < openFiles.length) set({ activeFileIndex: index, editorContent: openFiles[index].content }); },
  updateFileContent: (path, content) => { const { openFiles, activeFileIndex } = get(); const newFiles = openFiles.map(f => f.path === path ? { ...f, content, isDirty: true } : f); set({ openFiles: newFiles }); if (activeFileIndex >= 0 && newFiles[activeFileIndex]?.path === path) set({ editorContent: content }); },
  setEditorContent: (content) => set({ editorContent: content }), setCursorPosition: (line, column) => set({ cursorPosition: { line, column } }), setSelectedText: (text) => set({ selectedText: text }),
  setDiagnosticCounts: (errors, warnings) => set({ diagnosticCounts: { errors, warnings } }),
  setOllamaStatus: (running) => set({ isOllamaRunning: running }), setModels: (models) => set({ models }), setSelectedModel: (model) => set({ selectedModel: model }),
  addChatMessage: (message) => set(state => ({ chatMessages: [...state.chatMessages, message] })),
  upsertChatMessage: (message) => set(state => {
    const existingIndex = state.chatMessages.findIndex((entry) => entry.id === message.id);
    if (existingIndex === -1) {
      return { chatMessages: [...state.chatMessages, message] };
    }

    const chatMessages = [...state.chatMessages];
    chatMessages[existingIndex] = { ...chatMessages[existingIndex], ...message };
    return { chatMessages };
  }),
  clearChatMessages: () => set({ chatMessages: [] }), setAiLoading: (loading) => set({ isAiLoading: loading }),
  setTerminalOutput: (output) => set({ terminalOutput: output }), appendTerminalOutput: (output) => set(state => ({ terminalOutput: state.terminalOutput + output })), setGitStatus: (status) => set({ gitStatus: status }),
  setSupportedLanguages: (languages) => set({ supportedLanguages: languages }),
  setSidebarWidth: (width) => set({ sidebarWidth: width }), setChatWidth: (width) => set({ chatWidth: width }), toggleChat: () => set(state => ({ showChat: !state.showChat })), toggleTerminal: () => set(state => ({ showTerminal: !state.showTerminal })), setTerminalHeight: (height) => set({ terminalHeight: height }),
  setShowTerminal: (show) => set({ showTerminal: show }), setShowChat: (show) => set({ showChat: show }),
  
  // Embedded LLM actions
  setHardwareInfo: (info) => set({ hardwareInfo: info }),
  setLocalModels: (models) => set({ localModels: models }),
  setEmbeddedLLMReady: (ready) => set({ isEmbeddedLLMReady: ready }),
  setEmbeddedLLMLoading: (loading) => set({ isEmbeddedLLMLoading: loading }),
  setSelectedLocalModel: (model) => set({ selectedLocalModel: model }),
  updateInferenceStats: (tokens, timeMs) => set(state => {
    const newTotalTokens = state.inferenceStats.totalTokens + tokens;
    const newTotalTime = state.inferenceStats.totalTime + timeMs;
    return {
      inferenceStats: {
        totalTokens: newTotalTokens,
        totalTime: newTotalTime,
        avgTokensPerSecond: newTotalTime > 0 ? (newTotalTokens / (newTotalTime / 1000)) : 0
      }
    };
  }),
  
  // New actions
  setEditorOptions: (options) => set(state => ({
    settings: { ...state.settings, editorOptions: { ...state.settings.editorOptions, ...options } }
  })),
  setTheme: (theme) => set(state => ({ settings: { ...state.settings, theme } })),
  setKeybindings: (keybindings) => set(state => ({ settings: { ...state.settings, keybindings } })),
  setCurrentScope: (scope) => set({ currentScope: scope }),
  setSearchResults: (results) => set({ searchResults: results }),
  setIsSearching: (searching) => set({ isSearching: searching }),
  addBreakpoint: (breakpoint) => set(state => ({ breakpoints: [...state.breakpoints, breakpoint] })),
  removeBreakpoint: (id) => set(state => ({ breakpoints: state.breakpoints.filter(b => b.id !== id) })),
  toggleBreakpoint: (id) => set(state => ({
    breakpoints: state.breakpoints.map(b => b.id === id ? { ...b, enabled: !b.enabled } : b)
  })),
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
  setSymbolSearchOpen: (open) => set({ symbolSearchOpen: open }),
  setGlobalSearchOpen: (open) => set({ globalSearchOpen: open }),
  setInlineChatOpen: (open, position) => set({ inlineChatOpen: open, inlineChatPosition: position || null }),
  addRecentCommand: (command) => set(state => ({
    recentCommands: [command, ...state.recentCommands.filter(c => c !== command)].slice(0, 20)
  })),
  addRecentFile: (path) => set(state => ({
    recentFiles: [path, ...state.recentFiles.filter(f => f !== path)].slice(0, 20)
  })),
  pinFile: (path) => set(state => ({
    openFiles: state.openFiles.map(f => f.path === path ? { ...f, isPinned: true } : f)
  })),
  unpinFile: (path) => set(state => ({
    openFiles: state.openFiles.map(f => f.path === path ? { ...f, isPinned: false } : f)
  })),
  
  // Ghost text actions
  setGhostTextCompletion: (completion) => set({ ghostTextCompletion: completion }),
  updateGhostTextCompletion: (text) => set(state => ({
    ghostTextCompletion: state.ghostTextCompletion 
      ? { ...state.ghostTextCompletion, text }
      : null
  })),
  setGhostTextProcessing: (processing) => set({ isGhostTextProcessing: processing }),
  updateGhostTextCacheStats: (stats) => set(state => {
    const newStats = { ...state.ghostTextCacheStats, ...stats };
    if (newStats.hits + newStats.misses > 0) {
      newStats.hitRate = newStats.hits / (newStats.hits + newStats.misses);
    }
    return { ghostTextCacheStats: newStats };
  }),
  setGhostTextConfig: (config) => set(state => ({
    ghostTextConfig: { ...state.ghostTextConfig, ...config }
  })),
  clearGhostText: () => set({ ghostTextCompletion: null, isGhostTextProcessing: false }),
  
  // Editor Groups / Split Panes actions
  createEditorGroup: (id) => {
    const groupId = id || `group-${Date.now()}`;
    set(state => ({
      editorGroups: [...state.editorGroups, { id: groupId, tabs: [], activeTabIndex: -1 }]
    }));
    return groupId;
  },
  
  closeEditorGroup: (id) => set(state => {
    if (state.editorGroups.length <= 1) return state; // Don't close last group
    const newGroups = state.editorGroups.filter(g => g.id !== id);
    const newActiveGroupId = state.activeGroupId === id ? newGroups[0].id : state.activeGroupId;
    return {
      editorGroups: newGroups,
      activeGroupId: newActiveGroupId,
      splitDirection: newGroups.length === 1 ? 'none' : state.splitDirection
    };
  }),
  
  setActiveGroup: (id) => set({ activeGroupId: id }),
  
  addTabToGroup: (groupId, tab) => set(state => {
    const groups = state.editorGroups.map(g => {
      if (g.id !== groupId) return g;
      const existingIndex = g.tabs.findIndex(t => t.path === tab.path);
      if (existingIndex >= 0) {
        return { ...g, activeTabIndex: existingIndex };
      }
      return { ...g, tabs: [...g.tabs, tab], activeTabIndex: g.tabs.length };
    });
    return { editorGroups: groups };
  }),
  
  removeTabFromGroup: (groupId, tabIndex) => set(state => {
    const groups = state.editorGroups.map(g => {
      if (g.id !== groupId) return g;
      const newTabs = g.tabs.filter((_, i) => i !== tabIndex);
      let newActiveIndex = g.activeTabIndex;
      if (newTabs.length === 0) newActiveIndex = -1;
      else if (g.activeTabIndex >= newTabs.length) newActiveIndex = newTabs.length - 1;
      else if (tabIndex < g.activeTabIndex) newActiveIndex = g.activeTabIndex - 1;
      return { ...g, tabs: newTabs, activeTabIndex: newActiveIndex };
    });
    return { editorGroups: groups };
  }),
  
  setActiveTabInGroup: (groupId, tabIndex) => set(state => ({
    editorGroups: state.editorGroups.map(g =>
      g.id === groupId ? { ...g, activeTabIndex: tabIndex } : g
    )
  })),
  
  moveTabBetweenGroups: (fromGroupId, fromIndex, toGroupId, toIndex) => set(state => {
    const fromGroup = state.editorGroups.find(g => g.id === fromGroupId);
    if (!fromGroup) return state;
    
    const tab = fromGroup.tabs[fromIndex];
    if (!tab) return state;
    
    const groups = state.editorGroups.map(g => {
      if (g.id === fromGroupId) {
        const newTabs = g.tabs.filter((_, i) => i !== fromIndex);
        let newActiveIndex = g.activeTabIndex;
        if (g.activeTabIndex >= newTabs.length) newActiveIndex = Math.max(0, newTabs.length - 1);
        return { ...g, tabs: newTabs, activeTabIndex: newActiveIndex };
      }
      if (g.id === toGroupId) {
        const newTabs = [...g.tabs.slice(0, toIndex), tab, ...g.tabs.slice(toIndex)];
        return { ...g, tabs: newTabs, activeTabIndex: toIndex };
      }
      return g;
    });
    
    return { editorGroups: groups, activeGroupId: toGroupId, draggedTab: null };
  }),
  
  setSplitDirection: (direction) => set({ splitDirection: direction }),
  
  setDraggedTab: (dragInfo) => set({ draggedTab: dragInfo }),
  
  updateGroupSize: (groupId, width, height) => set(state => ({
    editorGroups: state.editorGroups.map(g =>
      g.id === groupId ? { ...g, width, height } : g
    )
  })),
  
  saveEditorViewState: (groupId, viewState) => set(state => {
    const group = state.editorGroups.find(g => g.id === groupId);
    if (!group || group.activeTabIndex < 0) return state;
    
    const groups = state.editorGroups.map(g => {
      if (g.id !== groupId) return g;
      const tabs = g.tabs.map((t, i) =>
        i === g.activeTabIndex ? { ...t, viewState } : t
      );
      return { ...g, tabs };
    });
    
    return { editorGroups: groups };
  }),
  
  // Multi-cursor history actions
  pushCursorOperation: (operation) => set(state => {
    const newHistory = [...state.cursorHistory, operation];
    if (newHistory.length > state.maxCursorHistory) {
      newHistory.shift();
    }
    return { cursorHistory: newHistory };
  }),
  
  popCursorOperation: () => {
    const state = get();
    if (state.cursorHistory.length === 0) return undefined;
    const operation = state.cursorHistory[state.cursorHistory.length - 1];
    set({ cursorHistory: state.cursorHistory.slice(0, -1) });
    return operation;
  },
  
  clearCursorHistory: () => set({ cursorHistory: [] }),
  
  // Minimap actions
  setMinimapVisible: (visible) => set({ minimapVisible: visible }),
  
  setMinimapScale: (scale) => set({ minimapScale: scale }),
  
  // Autopilot actions
  setAutopilotMode: (mode) => set({ autopilotMode: mode }),
  setAgentRunning: (running) => set({ isAgentRunning: running }),
  
  // Checkpoint actions
  addCheckpoint: (checkpoint) => set(state => ({
    checkpoints: [...state.checkpoints, checkpoint]
  })),
  removeCheckpoint: (id) => set(state => ({
    checkpoints: state.checkpoints.filter(c => c.id !== id)
  })),
  clearCheckpoints: () => set({ checkpoints: [] }),
  
  // Project rules actions
  setProjectRules: (rules) => set({ projectRules: rules }),
  updateProjectRule: (id, updates) => set(state => ({
    projectRules: state.projectRules.map(r =>
      r.id === id ? { ...r, ...updates } : r
    )
  })),
}));
