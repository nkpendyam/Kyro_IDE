'use client';

import React, { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useKyroStore } from '@/store/kyroStore';
import { X } from 'lucide-react';
import type { Terminal as XTerminal } from 'xterm';
import type { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';

export function TerminalPanel() {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const terminalCleanupRef = useRef<(() => void) | null>(null);
  const [isClient, setIsClient] = useState(false);
  const { projectPath, setShowTerminal } = useKyroStore();
  
  useEffect(() => { setIsClient(true); }, []);
  
  useEffect(() => {
    if (!isClient || !terminalRef.current) return;
    const initTerminal = async () => {
      try {
        terminalCleanupRef.current?.();
        const { Terminal } = await import('xterm');
        const { FitAddon } = await import('xterm-addon-fit');
        const term = new Terminal({ cursorBlink: true, fontSize: 13, fontFamily: 'JetBrains Mono, monospace',
          theme: { background: '#0D1117', foreground: '#C9D1D9', cursor: '#58A6FF', selectionBackground: '#264F78' } });
        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        term.open(terminalRef.current!);
        fitAddon.fit();
        xtermRef.current = term;
        fitAddonRef.current = fitAddon;
        invoke('create_terminal', { id: 'main', cwd: projectPath || undefined }).catch(console.error);
        term.onData((data: string) => { invoke('write_to_terminal', { id: 'main', data }).catch(console.error); });
        const handleResize = () => { fitAddon.fit(); invoke('resize_terminal', { id: 'main', cols: term.cols, rows: term.rows }).catch(console.error); };
        window.addEventListener('resize', handleResize);
        setTimeout(handleResize, 100);
        terminalCleanupRef.current = () => {
          window.removeEventListener('resize', handleResize);
          invoke('kill_terminal', { id: 'main' }).catch(console.error);
          term.dispose();
          xtermRef.current = null;
          fitAddonRef.current = null;
        };
      } catch (error) { console.error('Failed to initialize terminal:', error); }
    };
    initTerminal();
    return () => {
      terminalCleanupRef.current?.();
      terminalCleanupRef.current = null;
    };
  }, [isClient, projectPath]);
  
  return (
    <div className="flex flex-col h-full">
      <div className="h-8 bg-[#161b22] border-b border-[#30363d] flex items-center px-2 justify-between">
        <span className="text-xs font-medium text-[#8b949e]">Terminal</span>
        <button onClick={() => setShowTerminal(false)} className="p-1 hover:bg-[#21262d] rounded text-[#8b949e] hover:text-[#c9d1d9]" title="Close Terminal"><X size={14} /></button>
      </div>
      <div ref={terminalRef} className="flex-1 p-2 overflow-hidden bg-[#0d1117]" style={{ minHeight: 0 }} />
    </div>
  );
}
