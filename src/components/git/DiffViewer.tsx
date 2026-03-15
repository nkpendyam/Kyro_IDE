'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ChevronDown, ChevronUp, ChevronRight, Plus, Minus, File, RefreshCw } from 'lucide-react';

interface DiffLine {
  oldLineNumber: number | null;
  newLineNumber: number | null;
  type: 'unchanged' | 'added' | 'removed';
  content: string;
}

interface DiffHunk {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: DiffLine[];
}

interface GitDiffHunkResponse {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  lines: Array<{
    old_lineno: number | null;
    new_lineno: number | null;
    origin: string;
    content: string;
  }>;
}

interface DiffViewerProps {
  filePath?: string;
  mode: 'inline' | 'side-by-side';
}

export function DiffViewer({ filePath, mode = 'inline' }: DiffViewerProps) {
  const [diff, setDiff] = useState<DiffHunk[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [expandedHunks, setExpandedHunks] = useState<Set<number>>(new Set([0]));

  // Load diff
  useEffect(() => {
    if (!filePath) {
      setDiff([]);
      return;
    }

    const loadDiff = async () => {
      setIsLoading(true);
      try {
        const diffResult = await invoke<GitDiffHunkResponse[]>('git_diff_file', { path: filePath });
        setDiff(diffResult.map((hunk) => ({
          oldStart: hunk.old_start,
          oldLines: hunk.old_lines,
          newStart: hunk.new_start,
          newLines: hunk.new_lines,
          lines: hunk.lines.map((line) => ({
            oldLineNumber: line.old_lineno,
            newLineNumber: line.new_lineno,
            type: line.origin === '+' ? 'added' : line.origin === '-' ? 'removed' : 'unchanged',
            content: line.content,
          })),
        })));
      } catch (error) {
        console.error('Error loading diff:', error);
        setDiff([]);
      } finally {
        setIsLoading(false);
      }
    };

    loadDiff();
  }, [filePath]);

  // Toggle hunk expansion
  const toggleHunk = (index: number) => {
    setExpandedHunks(prev => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  };

  // Calculate stats
  const stats = useMemo(() => {
    let added = 0;
    let removed = 0;
    
    diff.forEach(hunk => {
      hunk.lines.forEach(line => {
        if (line.type === 'added') added++;
        if (line.type === 'removed') removed++;
      });
    });

    return { added, removed };
  }, [diff]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-32">
        <RefreshCw size={20} className="animate-spin text-[#8b949e]" />
      </div>
    );
  }

  if (diff.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-32 text-[#8b949e]">
        <File size={24} className="mb-2" />
        <span>No changes</span>
      </div>
    );
  }

  // Inline diff view
  if (mode === 'inline') {
    return (
      <div className="font-mono text-xs">
        {/* Stats */}
        <div className="flex items-center gap-2 px-4 py-2 border-b border-[#30363d] bg-[#161b22]">
          <span className="text-[#3fb950]">+{stats.added}</span>
          <span className="text-[#f85149]">-{stats.removed}</span>
        </div>

        {/* Diff hunks */}
        {diff.map((hunk, hunkIndex) => (
          <div key={hunkIndex}>
            {/* Hunk header */}
            <button
              onClick={() => toggleHunk(hunkIndex)}
              className="w-full flex items-center px-4 py-1 bg-[#161b22] border-b border-[#30363d] text-[#8b949e] hover:bg-[#21262d]"
            >
              {expandedHunks.has(hunkIndex) ? <ChevronDown size={12} /> : <ChevronRight size={12} className="mr-1" />}
              <span className="ml-1">@@ -{hunk.oldStart},{hunk.oldLines} +{hunk.newStart},{hunk.newLines} @@</span>
            </button>

            {/* Lines */}
            {expandedHunks.has(hunkIndex) && hunk.lines.map((line, lineIndex) => (
              <div
                key={lineIndex}
                className={`flex ${
                  line.type === 'added' ? 'bg-[#2ea04326]' :
                  line.type === 'removed' ? 'bg-[#f8514926]' :
                  ''
                }`}
              >
                {/* Line numbers */}
                <span className="w-10 text-right pr-2 text-[#484f58] select-none border-r border-[#30363d]">
                  {line.oldLineNumber ?? ''}
                </span>
                <span className="w-10 text-right pr-2 text-[#484f58] select-none border-r border-[#30363d]">
                  {line.newLineNumber ?? ''}
                </span>

                {/* Diff indicator */}
                <span className={`w-6 text-center select-none ${
                  line.type === 'added' ? 'text-[#3fb950]' :
                  line.type === 'removed' ? 'text-[#f85149]' :
                  'text-[#484f58]'
                }`}>
                  {line.type === 'added' ? '+' : line.type === 'removed' ? '-' : ' '}
                </span>

                {/* Content */}
                <span className={`flex-1 px-2 whitespace-pre ${
                  line.type === 'added' ? 'text-[#3fb950]' :
                  line.type === 'removed' ? 'text-[#f85149]' :
                  'text-[#c9d1d9]'
                }`}>
                  {line.content}
                </span>
              </div>
            ))}
          </div>
        ))}
      </div>
    );
  }

  // Side-by-side diff view
  return (
    <div className="flex h-full font-mono text-xs">
      {/* Stats */}
      <div className="absolute top-0 left-0 right-0 flex items-center gap-2 px-4 py-2 border-b border-[#30363d] bg-[#161b22] z-10">
        <span className="text-[#3fb950]">+{stats.added}</span>
        <span className="text-[#f85149]">-{stats.removed}</span>
      </div>

      {/* Side-by-side container */}
      <div className="flex flex-1 pt-10">
        {/* Old (left) side */}
        <div className="flex-1 border-r border-[#30363d] overflow-auto">
          <div className="sticky top-0 bg-[#161b22] border-b border-[#30363d] px-4 py-1 text-[#8b949e]">
            Original
          </div>
          {diff.flatMap((hunk, hunkIndex) => 
            expandedHunks.has(hunkIndex) ? hunk.lines.map((line, lineIndex) => (
              <div
                key={`old-${hunkIndex}-${lineIndex}`}
                className={`flex ${
                  line.type === 'removed' ? 'bg-[#f8514926]' : ''
                }`}
              >
                <span className="w-10 text-right pr-2 text-[#484f58] select-none">
                  {line.oldLineNumber ?? ''}
                </span>
                <span className={`w-6 text-center select-none ${
                  line.type === 'removed' ? 'text-[#f85149]' : 'text-[#484f58]'
                }`}>
                  {line.type === 'removed' ? '-' : ' '}
                </span>
                <span className={`flex-1 px-2 whitespace-pre ${
                  line.type === 'removed' ? 'text-[#f85149]' : 'text-[#c9d1d9]'
                }`}>
                  {line.type !== 'added' ? line.content : ''}
                </span>
              </div>
            )) : []
          )}
        </div>

        {/* New (right) side */}
        <div className="flex-1 overflow-auto">
          <div className="sticky top-0 bg-[#161b22] border-b border-[#30363d] px-4 py-1 text-[#8b949e]">
            Modified
          </div>
          {diff.flatMap((hunk, hunkIndex) => 
            expandedHunks.has(hunkIndex) ? hunk.lines.map((line, lineIndex) => (
              <div
                key={`new-${hunkIndex}-${lineIndex}`}
                className={`flex ${
                  line.type === 'added' ? 'bg-[#2ea04326]' : ''
                }`}
              >
                <span className="w-10 text-right pr-2 text-[#484f58] select-none">
                  {line.newLineNumber ?? ''}
                </span>
                <span className={`w-6 text-center select-none ${
                  line.type === 'added' ? 'text-[#3fb950]' : 'text-[#484f58]'
                }`}>
                  {line.type === 'added' ? '+' : ' '}
                </span>
                <span className={`flex-1 px-2 whitespace-pre ${
                  line.type === 'added' ? 'text-[#3fb950]' : 'text-[#c9d1d9]'
                }`}>
                  {line.type !== 'removed' ? line.content : ''}
                </span>
              </div>
            )) : []
          )}
        </div>
      </div>
    </div>
  );
}

