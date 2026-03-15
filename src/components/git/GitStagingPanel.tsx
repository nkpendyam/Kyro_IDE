'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { 
  GitBranch, Plus, Minus, Check, X, ChevronDown, ChevronRight,
  RefreshCw, FileCode, Trash2, RotateCcw
} from 'lucide-react';

// Git file status
interface GitFile {
  path: string;
  status: 'modified' | 'added' | 'deleted' | 'renamed' | 'untracked';
  staged: boolean;
  oldPath?: string; // For renames
}

// Diff hunk
interface DiffHunk {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  header: string;
  lines: DiffLine[];
}

interface GitDiffHunkResponse {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  header: string;
  lines: Array<{
    old_lineno: number | null;
    new_lineno: number | null;
    origin: string;
    content: string;
  }>;
}

// Diff line
interface DiffLine {
  type: 'context' | 'add' | 'delete';
  oldLine?: number;
  newLine?: number;
  content: string;
  selected: boolean;
}

// Props
interface GitStagingPanelProps {
  projectPath: string;
  onFileSelect?: (path: string) => void;
}

// File status badge color
function getStatusColor(status: string): string {
  switch (status) {
    case 'added': return 'text-[#3fb950]';
    case 'modified': return 'text-[#d29922]';
    case 'deleted': return 'text-[#f85149]';
    case 'renamed': return 'text-[#a371f7]';
    case 'untracked': return 'text-[#8b949e]';
    default: return 'text-[#8b949e]';
  }
}

// Status letter
function getStatusLetter(status: string): string {
  switch (status) {
    case 'added': return 'A';
    case 'modified': return 'M';
    case 'deleted': return 'D';
    case 'renamed': return 'R';
    case 'untracked': return '?';
    default: return '?';
  }
}

export function GitStagingPanel({ projectPath, onFileSelect }: GitStagingPanelProps) {
  const [stagedFiles, setStagedFiles] = useState<GitFile[]>([]);
  const [unstagedFiles, setUnstagedFiles] = useState<GitFile[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['staged', 'unstaged']));
  const [commitMessage, setCommitMessage] = useState('');
  const [diffHunks, setDiffHunks] = useState<DiffHunk[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  // Fetch git status
  const fetchStatus = useCallback(async () => {
    if (!projectPath) return;
    
    setIsLoading(true);
    try {
      const status = await invoke<{
        staged: Array<{ path: string; status: string }>;
        unstaged: Array<{ path: string; status: string }>;
        untracked: string[];
      }>('git_status', { path: projectPath });

      const staged: GitFile[] = status.staged.map(f => ({
        path: f.path,
        status: f.status as GitFile['status'],
        staged: true,
      }));

      const unstaged: GitFile[] = [
        ...status.unstaged.map(f => ({
          path: f.path,
          status: f.status as GitFile['status'],
          staged: false,
        })),
        ...status.untracked.map(path => ({
          path,
          status: 'untracked' as const,
          staged: false,
        })),
      ];

      setStagedFiles(staged);
      setUnstagedFiles(unstaged);
    } catch (e) {
      console.error('Failed to fetch git status:', e);
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  // Stage a file
  const stageFile = async (filePath: string) => {
    try {
      await invoke('git_stage', { projectPath, filePath });
      await fetchStatus();
    } catch (e) {
      console.error('Failed to stage file:', e);
    }
  };

  // Unstage a file
  const unstageFile = async (filePath: string) => {
    try {
      await invoke('git_unstage', { projectPath, filePath });
      await fetchStatus();
    } catch (e) {
      console.error('Failed to unstage file:', e);
    }
  };

  // Stage all files
  const stageAll = async () => {
    try {
      await invoke('git_stage_all', { projectPath });
      await fetchStatus();
    } catch (e) {
      console.error('Failed to stage all:', e);
    }
  };

  // Unstage all files
  const unstageAll = async () => {
    try {
      await invoke('git_unstage_all', { projectPath });
      await fetchStatus();
    } catch (e) {
      console.error('Failed to unstage all:', e);
    }
  };

  // Discard changes
  const discardChanges = async (filePath: string) => {
    try {
      await invoke('git_discard', { projectPath, filePath });
      await fetchStatus();
    } catch (e) {
      console.error('Failed to discard changes:', e);
    }
  };

  // Stage hunk
  const stageHunk = async (filePath: string, hunkIndex: number) => {
    try {
      await invoke('git_stage_hunk', { projectPath, filePath, hunkIndex });
      // Refresh diff
      selectFile(filePath);
    } catch (e) {
      console.error('Failed to stage hunk:', e);
    }
  };

  // Select file to view diff
  const selectFile = async (filePath: string) => {
    setSelectedFile(filePath);
    onFileSelect?.(filePath);
    
    try {
      const hunks = await invoke<GitDiffHunkResponse[]>('git_diff_file', {
        path: filePath,
      });
      setDiffHunks(hunks.map((hunk) => ({
        oldStart: hunk.old_start,
        oldLines: hunk.old_lines,
        newStart: hunk.new_start,
        newLines: hunk.new_lines,
        header: hunk.header,
        lines: hunk.lines.map((line) => ({
          type: line.origin === '+' ? 'add' : line.origin === '-' ? 'delete' : 'context',
          oldLine: line.old_lineno ?? undefined,
          newLine: line.new_lineno ?? undefined,
          content: line.content,
          selected: false,
        })),
      })));
    } catch (e) {
      console.error('Failed to get diff:', e);
      setDiffHunks([]);
    }
  };

  // Commit
  const commit = async () => {
    if (!commitMessage.trim() || stagedFiles.length === 0) return;
    
    try {
      await invoke('git_commit', { 
        projectPath, 
        message: commitMessage 
      });
      setCommitMessage('');
      await fetchStatus();
    } catch (e) {
      console.error('Failed to commit:', e);
    }
  };

  // Toggle section
  const toggleSection = (section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) {
        next.delete(section);
      } else {
        next.add(section);
      }
      return next;
    });
  };

  // Render file item
  const renderFile = (file: GitFile, isStaged: boolean) => (
    <div
      key={file.path}
      className={`group flex items-center gap-2 px-2 py-1 hover:bg-[#21262d] cursor-pointer ${
        selectedFile === file.path ? 'bg-[#21262d]' : ''
      }`}
      onClick={() => selectFile(file.path)}
    >
      {/* Status badge */}
      <span className={`w-4 text-center font-mono text-xs ${getStatusColor(file.status)}`}>
        {getStatusLetter(file.status)}
      </span>
      
      {/* File path */}
      <span className="flex-1 text-sm truncate text-[#c9d1d9]">
        {file.path.split('/').pop()}
      </span>
      
      {/* Directory hint */}
      <span className="text-xs text-[#8b949e] truncate max-w-25">
        {file.path.includes('/') && file.path.substring(0, file.path.lastIndexOf('/'))}
      </span>
      
      {/* Action buttons */}
      <div className="flex gap-1 opacity-0 group-hover:opacity-100">
        {isStaged ? (
          <button
            onClick={(e) => { e.stopPropagation(); unstageFile(file.path); }}
            className="p-1 hover:bg-[#30363d] rounded text-[#8b949e] hover:text-[#c9d1d9]"
            title="Unstage"
          >
            <Minus size={12} />
          </button>
        ) : (
          <>
            <button
              onClick={(e) => { e.stopPropagation(); stageFile(file.path); }}
              className="p-1 hover:bg-[#30363d] rounded text-[#8b949e] hover:text-[#c9d1d9]"
              title="Stage"
            >
              <Plus size={12} />
            </button>
            {file.status !== 'untracked' && (
              <button
                onClick={(e) => { e.stopPropagation(); discardChanges(file.path); }}
                className="p-1 hover:bg-[#30363d] rounded text-[#8b949e] hover:text-[#f85149]"
                title="Discard changes"
              >
                <RotateCcw size={12} />
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );

  return (
    <div className="flex flex-col h-full bg-[#0d1117] text-[#c9d1d9]">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-[#30363d]">
        <div className="flex items-center gap-2">
          <GitBranch size={14} className="text-[#8b949e]" />
          <span className="text-sm font-medium">Source Control</span>
          {stagedFiles.length + unstagedFiles.length > 0 && (
            <span className="px-1.5 py-0.5 bg-[#21262d] rounded text-xs text-[#8b949e]">
              {stagedFiles.length + unstagedFiles.length}
            </span>
          )}
        </div>
        <button
          onClick={fetchStatus}
          disabled={isLoading}
          className="p-1 hover:bg-[#21262d] rounded text-[#8b949e] hover:text-[#c9d1d9]"
        >
          <RefreshCw size={14} className={isLoading ? 'animate-spin' : ''} />
        </button>
      </div>

      {/* Commit input */}
      {stagedFiles.length > 0 && (
        <div className="p-2 border-b border-[#30363d]">
          <textarea
            value={commitMessage}
            onChange={(e) => setCommitMessage(e.target.value)}
            placeholder="Commit message..."
            className="w-full px-2 py-1 bg-[#0d1117] border border-[#30363d] rounded text-sm resize-none focus:outline-none focus:border-[#58a6ff]"
            rows={3}
          />
          <button
            onClick={commit}
            disabled={!commitMessage.trim()}
            className="mt-2 w-full py-1.5 bg-[#238636] hover:bg-[#2ea043] disabled:bg-[#21262d] disabled:text-[#8b949e] text-white text-sm rounded"
          >
            Commit {stagedFiles.length} file{stagedFiles.length !== 1 ? 's' : ''}
          </button>
        </div>
      )}

      {/* File lists */}
      <div className="flex-1 overflow-y-auto">
        {/* Staged changes */}
        {stagedFiles.length > 0 && (
          <div className="border-b border-[#30363d]">
            <button
              onClick={() => toggleSection('staged')}
              className="w-full flex items-center gap-1 px-2 py-1.5 hover:bg-[#21262d] text-left"
            >
              {expandedSections.has('staged') ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
              <span className="text-xs font-medium text-[#8b949e]">Staged Changes</span>
              <span className="text-xs text-[#8b949e]">({stagedFiles.length})</span>
              <button
                onClick={(e) => { e.stopPropagation(); unstageAll(); }}
                className="ml-auto text-xs text-[#58a6ff] hover:underline"
              >
                Unstage All
              </button>
            </button>
            {expandedSections.has('staged') && (
              <div className="pb-1">
                {stagedFiles.map(file => renderFile(file, true))}
              </div>
            )}
          </div>
        )}

        {/* Unstaged changes */}
        {unstagedFiles.length > 0 && (
          <div>
            <button
              onClick={() => toggleSection('unstaged')}
              className="w-full flex items-center gap-1 px-2 py-1.5 hover:bg-[#21262d] text-left"
            >
              {expandedSections.has('unstaged') ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
              <span className="text-xs font-medium text-[#8b949e]">Changes</span>
              <span className="text-xs text-[#8b949e]">({unstagedFiles.length})</span>
              <button
                onClick={(e) => { e.stopPropagation(); stageAll(); }}
                className="ml-auto text-xs text-[#58a6ff] hover:underline"
              >
                Stage All
              </button>
            </button>
            {expandedSections.has('unstaged') && (
              <div className="pb-1">
                {unstagedFiles.map(file => renderFile(file, false))}
              </div>
            )}
          </div>
        )}

        {/* Empty state */}
        {stagedFiles.length === 0 && unstagedFiles.length === 0 && !isLoading && (
          <div className="p-4 text-center text-[#8b949e]">
            <GitBranch className="mx-auto mb-2 opacity-50" size={24} />
            <p className="text-sm">No changes detected</p>
          </div>
        )}
      </div>

      {/* Diff view for selected file */}
      {selectedFile && diffHunks.length > 0 && (
        <div className="border-t border-[#30363d] h-48 overflow-y-auto">
          <div className="flex items-center justify-between px-2 py-1 bg-[#161b22] sticky top-0">
            <span className="text-xs text-[#8b949e] truncate">{selectedFile}</span>
            <button
              onClick={() => setSelectedFile(null)}
              className="p-1 hover:bg-[#21262d] rounded text-[#8b949e]"
            >
              <X size={12} />
            </button>
          </div>
          
          {diffHunks.map((hunk, idx) => (
            <div key={idx}>
              {/* Hunk header */}
              <div className="flex items-center justify-between px-2 py-0.5 bg-[#161b22] text-xs text-[#8b949e] font-mono">
                <span>{hunk.header}</span>
                <button
                  onClick={() => stageHunk(selectedFile, idx)}
                  className="px-1.5 py-0.5 bg-[#238636] hover:bg-[#2ea043] text-white rounded text-xs"
                  title="Stage hunk"
                >
                  Stage Hunk
                </button>
              </div>
              
              {/* Diff lines */}
              {hunk.lines.map((line, lineIdx) => (
                <div
                  key={lineIdx}
                  className={`flex font-mono text-xs ${
                    line.type === 'add' ? 'bg-[#3fb95020]' :
                    line.type === 'delete' ? 'bg-[#f8514920]' : ''
                  }`}
                >
                  <span className="w-8 text-right pr-2 text-[#484f58] select-none">
                    {line.oldLine ?? ''}
                  </span>
                  <span className="w-8 text-right pr-2 text-[#484f58] select-none">
                    {line.newLine ?? ''}
                  </span>
                  <span className={`w-4 text-center ${
                    line.type === 'add' ? 'text-[#3fb950]' :
                    line.type === 'delete' ? 'text-[#f85149]' : 'text-[#484f58]'
                  }`}>
                    {line.type === 'add' ? '+' : line.type === 'delete' ? '-' : ' '}
                  </span>
                  <span className="flex-1 px-1 whitespace-pre">{line.content}</span>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default GitStagingPanel;
