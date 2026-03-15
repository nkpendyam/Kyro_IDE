'use client';

import React, { useState, useEffect, useRef, useCallback } from 'react';
import { useKyroStore } from '@/store/kyroStore';
import { File, FolderOpen, Database, Terminal, Globe, GitBranch, MessageSquare } from 'lucide-react';

export interface MentionItem {
  type: '@file' | '@folder' | '@codebase' | '@terminal' | '@web' | '@git' | '@previous';
  label: string;
  description: string;
  icon: React.ReactNode;
  value: string;
}

const MENTION_TYPES: MentionItem[] = [
  { type: '@file', label: '@file', description: 'Reference a specific file', icon: <File size={14} />, value: '@file' },
  { type: '@folder', label: '@folder', description: 'Reference a folder', icon: <FolderOpen size={14} />, value: '@folder' },
  { type: '@codebase', label: '@codebase', description: 'Search entire codebase', icon: <Database size={14} />, value: '@codebase' },
  { type: '@terminal', label: '@terminal', description: 'Last terminal output', icon: <Terminal size={14} />, value: '@terminal' },
  { type: '@web', label: '@web', description: 'Search the web', icon: <Globe size={14} />, value: '@web' },
  { type: '@git', label: '@git', description: 'Git diff & history', icon: <GitBranch size={14} />, value: '@git' },
  { type: '@previous', label: '@previous', description: 'Previous conversation', icon: <MessageSquare size={14} />, value: '@previous' },
];

interface MentionAutocompleteProps {
  inputValue: string;
  cursorPosition: number;
  onSelect: (mention: MentionItem, filePath?: string) => void;
  onDismiss: () => void;
  visible: boolean;
}

export function MentionAutocomplete({ inputValue, cursorPosition, onSelect, onDismiss, visible }: MentionAutocompleteProps) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showFilePicker, setShowFilePicker] = useState(false);
  const [fileFilter, setFileFilter] = useState('');
  const listRef = useRef<HTMLDivElement>(null);
  const { openFiles, fileTree } = useKyroStore();

  // Extract the @ query from cursor position
  const getAtQuery = useCallback(() => {
    const textBeforeCursor = inputValue.slice(0, cursorPosition);
    const atMatch = textBeforeCursor.match(/@(\w*)$/);
    return atMatch ? atMatch[1].toLowerCase() : null;
  }, [inputValue, cursorPosition]);

  const atQuery = getAtQuery();

  // Filter mention types based on query
  const filteredMentions = atQuery !== null
    ? MENTION_TYPES.filter(m => m.type.slice(1).startsWith(atQuery))
    : [];

  // Collect files from the file tree
  const allFiles = React.useMemo(() => {
    if (!showFilePicker || !fileTree) return [];
    const results: string[] = [];
    const stack = [fileTree];
    while (stack.length > 0) {
      const node = stack.pop()!;
      if (!node.is_directory) {
        results.push(node.path);
      }
      if (node.children) {
        for (const child of node.children) {
          stack.push(child);
        }
      }
    }
    return results;
  }, [showFilePicker, fileTree]);
  const filteredFiles = allFiles.filter(f =>
    f.toLowerCase().includes(fileFilter.toLowerCase())
  ).slice(0, 15);

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      setSelectedIndex(0);
    }, 0);

    return () => clearTimeout(timeoutId);
  }, [atQuery]);

  const items = showFilePicker ? filteredFiles : filteredMentions;
  const effectiveSelectedIndex = items.length === 0 ? 0 : Math.min(selectedIndex, items.length - 1);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!visible || items.length === 0) return;

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((i) => (i + 1) % items.length);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((i) => (i - 1 + items.length) % items.length);
      } else if (e.key === 'Enter' || e.key === 'Tab') {
        e.preventDefault();
        if (showFilePicker) {
          const selectedFile = filteredFiles[effectiveSelectedIndex];
          if (selectedFile) {
            onSelect(MENTION_TYPES[0], selectedFile);
            setShowFilePicker(false);
          }
        } else {
          const selected = filteredMentions[effectiveSelectedIndex];
          if (selected) {
            if (selected.type === '@file') {
              setShowFilePicker(true);
              setFileFilter('');
              return;
            }
            onSelect(selected);
          }
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        setShowFilePicker(false);
        onDismiss();
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [visible, items, effectiveSelectedIndex, filteredMentions, filteredFiles, showFilePicker, onSelect, onDismiss]);

  if (!visible || (filteredMentions.length === 0 && !showFilePicker)) return null;

  return (
    <div
      ref={listRef}
      className="absolute bottom-full left-0 mb-1 w-64 bg-[#1c2128] border border-[#30363d] rounded-lg shadow-xl z-50 overflow-hidden"
    >
      {showFilePicker ? (
        <div>
          <div className="px-3 py-2 border-b border-[#30363d]">
            <input
              type="text"
              value={fileFilter}
              onChange={(e) => { setFileFilter(e.target.value); setSelectedIndex(0); }}
              placeholder="Type to filter files..."
              className="w-full bg-[#0d1117] border border-[#30363d] rounded px-2 py-1 text-xs text-[#c9d1d9] focus:outline-none focus:border-[#58a6ff]"
              autoFocus
            />
          </div>
          <div className="max-h-48 overflow-y-auto">
            {filteredFiles.map((filePath, index) => {
              const fileName = filePath.split(/[/\\]/).pop() || filePath;
              return (
                <button
                  key={filePath}
                  onClick={() => { onSelect(MENTION_TYPES[0], filePath); setShowFilePicker(false); }}
                  className={`w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left transition-colors ${
                    index === effectiveSelectedIndex ? 'bg-[#388bfd26] text-[#c9d1d9]' : 'text-[#8b949e] hover:bg-[#21262d]'
                  }`}
                >
                  <File size={12} className="shrink-0" />
                  <span className="truncate">{fileName}</span>
                </button>
              );
            })}
            {filteredFiles.length === 0 && (
              <div className="px-3 py-2 text-xs text-[#8b949e]">No files found</div>
            )}
          </div>
        </div>
      ) : (
        <div className="max-h-48 overflow-y-auto py-1">
          {filteredMentions.map((mention, index) => (
            <button
              key={mention.type}
              onClick={() => {
                if (mention.type === '@file') {
                  setShowFilePicker(true);
                  setFileFilter('');
                  return;
                }
                onSelect(mention);
              }}
              className={`w-full flex items-center gap-2 px-3 py-2 text-left transition-colors ${
                index === effectiveSelectedIndex ? 'bg-[#388bfd26] text-[#c9d1d9]' : 'text-[#8b949e] hover:bg-[#21262d]'
              }`}
            >
              <span className="shrink-0 text-[#58a6ff]">{mention.icon}</span>
              <div>
                <div className="text-xs font-medium">{mention.label}</div>
                <div className="text-[10px] opacity-60">{mention.description}</div>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * Parse @ mentions from a message string and return context-enriched content
 */
export function parseMentions(text: string): { cleanText: string; mentions: Array<{ type: string; value: string }> } {
  const mentionRegex = /@(file|folder|codebase|terminal|web|git|previous)(?:\(([^)]+)\))?/g;
  const mentions: Array<{ type: string; value: string }> = [];
  let cleanText = text;

  let match;
  while ((match = mentionRegex.exec(text)) !== null) {
    mentions.push({ type: match[1], value: match[2] || '' });
  }

  // Remove mention syntax from display text
  cleanText = text.replace(mentionRegex, (_, type, value) =>
    value ? `[${type}: ${value}]` : `[${type}]`
  );

  return { cleanText, mentions };
}
