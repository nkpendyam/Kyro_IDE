'use client';

import React, { useRef, useLayoutEffect, useEffect, useState, useCallback } from 'react';
import * as monaco from 'monaco-editor';
import { useKyroStore } from '@/store/kyroStore';

// Minimap configuration
interface MinimapConfig {
  visible: boolean;
  scale: number;
  showSlider: 'always' | 'mouseover' | 'hidden';
  renderCharacters: boolean;
  maxColumn: number;
  side: 'left' | 'right';
}

interface MinimapProps {
  editor: monaco.editor.IStandaloneCodeEditor | null;
  visible?: boolean;
  scale?: number;
  onToggle?: () => void;
}

// Custom Minimap overlay component for enhanced functionality
export function Minimap({ editor, visible = true, scale = 1, onToggle }: MinimapProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [viewportHeight, setViewportHeight] = useState(0);
  const [viewportTop, setViewportTop] = useState(0);
  const [contentHeight, setContentHeight] = useState(0);
  const [lineCount, setLineCount] = useState(0);
  const [isDragging, setIsDragging] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  
  // Update minimap state from editor
  const updateMinimapState = useCallback(() => {
    if (!editor) return;
    
    const layoutInfo = editor.getLayoutInfo();
    const model = editor.getModel();
    if (!model) return;
    
    const visibleRanges = editor.getVisibleRanges();
    const totalLines = model.getLineCount();
    const lineHeight = editor.getOption(monaco.editor.EditorOption.lineHeight);
    
    // Calculate viewport
    const editorHeight = editor.getContentHeight();
    const totalContentHeight = totalLines * lineHeight;
    
    if (visibleRanges.length > 0) {
      const startLine = visibleRanges[0].startLineNumber;
      const endLine = visibleRanges[visibleRanges.length - 1].endLineNumber;
      
      const viewportStart = (startLine / totalLines) * 100;
      const viewportEnd = (endLine / totalLines) * 100;
      const viewportSize = Math.max(5, viewportEnd - viewportStart);
      
      setViewportTop(viewportStart);
      setViewportHeight(viewportSize);
    }
    
    setContentHeight(totalContentHeight);
    setLineCount(totalLines);
  }, [editor]);
  
  // Subscribe to editor changes
  useLayoutEffect(() => {
    if (!editor) return;
    
    const disposables = [
      editor.onDidScrollChange(updateMinimapState),
      editor.onDidChangeModelContent(updateMinimapState),
      editor.onDidChangeModel(() => updateMinimapState()),
      editor.onDidLayoutChange(updateMinimapState),
    ];
    
    // Queue initial state update to happen after effect completes
    // This avoids synchronous setState in effect body
    const timeoutId = setTimeout(updateMinimapState, 0);
    
    return () => {
      clearTimeout(timeoutId);
      disposables.forEach(d => d.dispose());
    };
  }, [editor, updateMinimapState]);
  
  // Handle click-to-scroll
  const handleClick = useCallback((e: React.MouseEvent) => {
    if (!editor || !containerRef.current) return;
    
    const rect = containerRef.current.getBoundingClientRect();
    const clickPosition = (e.clientY - rect.top) / rect.height;
    const model = editor.getModel();
    if (!model) return;
    
    const targetLine = Math.round(clickPosition * model.getLineCount());
    editor.revealLineInCenter(targetLine);
  }, [editor]);
  
  // Handle drag-to-scroll
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (!editor) return;
    e.preventDefault();
    setIsDragging(true);
    
    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!containerRef.current) return;
      
      const rect = containerRef.current.getBoundingClientRect();
      const position = (moveEvent.clientY - rect.top) / rect.height;
      const model = editor.getModel();
      if (!model) return;
      
      const targetLine = Math.round(position * model.getLineCount());
      editor.revealLineInCenter(targetLine);
    };
    
    const handleMouseUp = () => {
      setIsDragging(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
    
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [editor]);
  
  if (!visible) return null;
  
  return (
    <div
      ref={containerRef}
      className={`relative w-20 h-full bg-[#0d111780] cursor-pointer transition-opacity ${isHovered ? 'opacity-100' : 'opacity-70'}`}
      onClick={handleClick}
      onMouseDown={handleMouseDown}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      style={{ 
        transform: `scaleX(${scale})`,
        transformOrigin: 'right'
      }}
    >
      {/* Viewport indicator */}
      <div
        className={`absolute left-0 right-0 bg-[#264f78] transition-all ${isDragging ? 'bg-[#58a6ff]' : ''} ${isHovered ? 'border border-[#58a6ff]' : ''}`}
        style={{
          top: `${viewportTop}%`,
          height: `${viewportHeight}%`,
          minHeight: '10px'
        }}
      />
      
      {/* Line decorations preview (simplified) */}
      <MinimapDecorations editor={editor} visible={isHovered} />
      
      {/* Current line indicator */}
      <MinimapCurrentLine editor={editor} />
      
      {/* Cursor position marker */}
      <MinimapCursor editor={editor} />
    </div>
  );
}

// Minimap line decorations
function MinimapDecorations({ editor, visible }: { editor: monaco.editor.IStandaloneCodeEditor | null; visible: boolean }) {
  const [decorations, setDecorations] = useState<Array<{ line: number; type: 'error' | 'warning' | 'info' }>>([]);
  
  useEffect(() => {
    if (!editor || !visible) return;
    
    const model = editor.getModel();
    if (!model) return;
    
    const updateDecorations = () => {
      const markers = monaco.editor.getModelMarkers({ resource: model.uri });
      const decs = markers.map(m => ({
        line: m.startLineNumber,
        type: m.severity === monaco.MarkerSeverity.Error ? 'error' as const 
          : m.severity === monaco.MarkerSeverity.Warning ? 'warning' as const 
          : 'info' as const
      }));
      setDecorations(decs.slice(0, 50)); // Limit for performance
    };
    
    updateDecorations();
    
    const disposable = monaco.editor.onDidChangeMarkers(updateDecorations);
    return () => disposable.dispose();
  }, [editor, visible]);
  
  if (!visible) return null;
  
  return (
    <div className="absolute inset-0 pointer-events-none">
      {decorations.map((dec, i) => (
        <div
          key={`${dec.line}-${i}`}
          className={`absolute left-0 w-1 h-1 rounded-full ${
            dec.type === 'error' ? 'bg-[#f85149]' 
            : dec.type === 'warning' ? 'bg-[#d29922]' 
            : 'bg-[#58a6ff]'
          }`}
          style={{ top: `${(dec.line / (editor?.getModel()?.getLineCount() || 1)) * 100}%` }}
        />
      ))}
    </div>
  );
}

// Current line highlight
function MinimapCurrentLine({ editor }: { editor: monaco.editor.IStandaloneCodeEditor | null }) {
  const [currentLine, setCurrentLine] = useState(1);
  const [totalLines, setTotalLines] = useState(1);
  
  useEffect(() => {
    if (!editor) return;
    
    const updateCurrentLine = () => {
      const position = editor.getPosition();
      const model = editor.getModel();
      if (position && model) {
        setCurrentLine(position.lineNumber);
        setTotalLines(model.getLineCount());
      }
    };
    
    updateCurrentLine();
    
    const disposable = editor.onDidChangeCursorPosition(updateCurrentLine);
    return () => disposable.dispose();
  }, [editor]);
  
  const position = (currentLine / totalLines) * 100;
  
  return (
    <div
      className="absolute left-0 right-0 h-px bg-[#58a6ff80] pointer-events-none"
      style={{ top: `${position}%` }}
    />
  );
}

// Cursor position in minimap
function MinimapCursor({ editor }: { editor: monaco.editor.IStandaloneCodeEditor | null }) {
  const [cursorPosition, setCursorPosition] = useState({ line: 1, column: 1 });
  const [totalLines, setTotalLines] = useState(1);
  const [lineContent, setLineContent] = useState('');
  
  useEffect(() => {
    if (!editor) return;
    
    const updateCursor = () => {
      const position = editor.getPosition();
      const model = editor.getModel();
      if (position && model) {
        setCursorPosition({ line: position.lineNumber, column: position.column });
        setTotalLines(model.getLineCount());
        setLineContent(model.getLineContent(position.lineNumber).substring(0, 20));
      }
    };
    
    updateCursor();
    
    const disposables = [
      editor.onDidChangeCursorPosition(updateCursor),
      editor.onDidChangeModelContent(updateCursor),
    ];
    
    return () => disposables.forEach(d => d.dispose());
  }, [editor]);
  
  return (
    <div
      className="absolute left-0 w-0.5 h-3 bg-[#58a6ff] pointer-events-none transition-all"
      style={{ 
        top: `${(cursorPosition.line / totalLines) * 100}%`,
        left: `${Math.min(90, (cursorPosition.column / 80) * 100)}%`
      }}
      title={`Line ${cursorPosition.line}, Column ${cursorPosition.column}`}
    />
  );
}

// Minimap toggle button component
export function MinimapToggle({ visible, onToggle }: { visible: boolean; onToggle: () => void }) {
  return (
    <button
      onClick={onToggle}
      className={`p-1.5 rounded hover:bg-[#21262d] transition-colors ${visible ? 'text-[#58a6ff]' : 'text-[#8b949e]'}`}
      title={visible ? 'Hide Minimap' : 'Show Minimap'}
    >
      <svg className="w-4 h-4" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
        <rect x="1" y="1" width="14" height="14" rx="1" />
        <rect x="2" y="2" width="3" height="1" fill="currentColor" opacity="0.5" />
        <rect x="2" y="4" width="4" height="0.5" fill="currentColor" opacity="0.3" />
        <rect x="2" y="5.5" width="2" height="0.5" fill="currentColor" opacity="0.3" />
        <rect x="2" y="7" width="5" height="0.5" fill="currentColor" opacity="0.3" />
        <rect x="2" y="8.5" width="3" height="0.5" fill="currentColor" opacity="0.3" />
        <rect x="2" y="10" width="4" height="0.5" fill="currentColor" opacity="0.3" />
        {!visible && (
          <line x1="1" y1="15" x2="15" y2="1" stroke="currentColor" strokeWidth="1.5" />
        )}
      </svg>
    </button>
  );
}

// Minimap scale slider
export function MinimapScaleSlider({ scale, onChange }: { scale: number; onChange: (scale: number) => void }) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-[#8b949e]">Minimap Scale</span>
      <input
        type="range"
        min="0.5"
        max="2"
        step="0.1"
        value={scale}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="w-20 h-1 bg-[#30363d] rounded appearance-none cursor-pointer
          [&::-webkit-slider-thumb]:appearance-none
          [&::-webkit-slider-thumb]:w-3
          [&::-webkit-slider-thumb]:h-3
          [&::-webkit-slider-thumb]:rounded-full
          [&::-webkit-slider-thumb]:bg-[#58a6ff]"
      />
      <span className="text-xs text-[#8b949e] w-8">{scale.toFixed(1)}x</span>
    </div>
  );
}

// Full minimap panel with controls
export function MinimapPanel({ editor }: { editor: monaco.editor.IStandaloneCodeEditor | null }) {
  const { minimapVisible, setMinimapVisible, minimapScale, setMinimapScale } = useMinimapStore();
  const [showControls, setShowControls] = useState(false);
  
  return (
    <div 
      className="relative h-full"
      onMouseEnter={() => setShowControls(true)}
      onMouseLeave={() => setShowControls(false)}
    >
      <Minimap 
        editor={editor} 
        visible={minimapVisible} 
        scale={minimapScale}
        onToggle={() => setMinimapVisible(!minimapVisible)}
      />
      
      {/* Controls overlay */}
      {showControls && minimapVisible && (
        <div className="absolute top-2 right-2 bg-[#161b22] border border-[#30363d] rounded p-2 shadow-lg z-10">
          <MinimapScaleSlider scale={minimapScale} onChange={setMinimapScale} />
        </div>
      )}
    </div>
  );
}

// Hook to access minimap state from Zustand store
function useMinimapStore() {
  const minimapVisible = useKyroStore(state => state.minimapVisible);
  const setMinimapVisible = useKyroStore(state => state.setMinimapVisible);
  const minimapScale = useKyroStore(state => state.minimapScale);
  const setMinimapScale = useKyroStore(state => state.setMinimapScale);
  
  return {
    minimapVisible,
    setMinimapVisible,
    minimapScale,
    setMinimapScale,
  };
}

// Hook for editor minimap integration
export function useEditorMinimap(editor: monaco.editor.IStandaloneCodeEditor | null) {
  const { minimapVisible, minimapScale } = useMinimapStore();
  
  useEffect(() => {
    if (!editor) return;
    
    editor.updateOptions({
      minimap: {
        enabled: minimapVisible,
        scale: minimapScale,
        showSlider: 'mouseover',
        renderCharacters: false,
        maxColumn: 80,
      }
    });
  }, [editor, minimapVisible, minimapScale]);
  
  return {
    isVisible: minimapVisible,
    scale: minimapScale,
  };
}

export default Minimap;
