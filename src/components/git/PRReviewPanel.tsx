'use client';

import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  AlertTriangle,
  CheckCircle2,
  FileCode2,
  GitBranch,
  Loader2,
  RefreshCw,
  ShieldAlert,
  Sparkles,
  Wand2,
} from 'lucide-react';

import { readFile, writeFile, detectLanguage, joinPath, normalizePath } from '@/lib/fileOperations';
import { reviewDiff, type DiffReviewResult } from '@/lib/tauri-commands';
import { useKyroStore } from '@/store/kyroStore';

interface DiffLine {
  old_lineno: number | null;
  new_lineno: number | null;
  origin: string;
  content: string;
}

interface DiffHunk {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  header: string;
  lines: DiffLine[];
}

interface FileDiff {
  file: string;
  status: string;
  additions: number;
  deletions: number;
  hunks: DiffHunk[];
}

interface GitStatusResponse {
  branch: string;
}

interface ReviewableFile extends FileDiff {
  source: 'staged' | 'unstaged' | 'both';
}

interface FixPreview {
  commentId: string;
  original: string;
  suggested: string;
}

interface PRReviewPanelProps {
  projectPath: string;
  onOpenFile?: (path: string) => void;
}

function mergeDiffs(unstaged: FileDiff[], staged: FileDiff[]): ReviewableFile[] {
  const merged = new Map<string, ReviewableFile>();

  unstaged.forEach((diff) => {
    merged.set(diff.file, { ...diff, source: 'unstaged' });
  });

  staged.forEach((diff) => {
    const existing = merged.get(diff.file);
    if (!existing) {
      merged.set(diff.file, { ...diff, source: 'staged' });
      return;
    }

    merged.set(diff.file, {
      ...existing,
      additions: existing.additions + diff.additions,
      deletions: existing.deletions + diff.deletions,
      hunks: [...existing.hunks, ...diff.hunks],
      source: 'both',
    });
  });

  return Array.from(merged.values()).sort((left, right) => {
    const leftScore = left.additions + left.deletions;
    const rightScore = right.additions + right.deletions;
    if (leftScore !== rightScore) {
      return rightScore - leftScore;
    }
    return left.file.localeCompare(right.file);
  });
}

function buildUnifiedDiff(diff: FileDiff): string {
  const lines = [
    `--- a/${diff.file}`,
    `+++ b/${diff.file}`,
  ];

  diff.hunks.forEach((hunk) => {
    lines.push(hunk.header || `@@ -${hunk.old_start},${hunk.old_lines} +${hunk.new_start},${hunk.new_lines} @@`);
    hunk.lines.forEach((line) => {
      const origin = line.origin || ' ';
      lines.push(`${origin}${line.content}`.replace(/\r?\n$/, ''));
    });
  });

  return lines.join('\n');
}

function sourceLabel(source: ReviewableFile['source']): string {
  if (source === 'both') {
    return 'Staged + Working Tree';
  }
  if (source === 'staged') {
    return 'Staged';
  }
  return 'Working Tree';
}

function riskTone(risk: string): string {
  if (risk === 'high') {
    return 'text-[#f85149] border-[#f85149]/30 bg-[#f85149]/10';
  }
  if (risk === 'medium') {
    return 'text-[#d29922] border-[#d29922]/30 bg-[#d29922]/10';
  }
  return 'text-[#3fb950] border-[#3fb950]/30 bg-[#3fb950]/10';
}

function commentTone(severity: string): string {
  if (severity === 'error') {
    return 'border-[#f85149]/30 bg-[#f85149]/10';
  }
  if (severity === 'warning') {
    return 'border-[#d29922]/30 bg-[#d29922]/10';
  }
  return 'border-[#30363d] bg-[#11161d]';
}

function extractSuggestedCode(response: string): string {
  const fenced = response.match(/```(?:[a-zA-Z0-9_+-]+)?\n([\s\S]*?)```/);
  if (fenced?.[1]) {
    return fenced[1].trim();
  }
  return response.trim();
}

export function PRReviewPanel({ projectPath, onOpenFile }: PRReviewPanelProps) {
  const selectedModel = useKyroStore((state) => state.selectedModel);
  const [branch, setBranch] = useState('HEAD');
  const [files, setFiles] = useState<ReviewableFile[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [reviews, setReviews] = useState<Record<string, DiffReviewResult>>({});
  const [fixPreview, setFixPreview] = useState<FixPreview | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isReviewing, setIsReviewing] = useState(false);
  const [isGeneratingFix, setIsGeneratingFix] = useState(false);
  const [isApplyingFix, setIsApplyingFix] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadReviewState = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const [status, unstaged, staged] = await Promise.all([
        invoke<GitStatusResponse>('git_status', { path: projectPath }),
        invoke<FileDiff[]>('git_diff', { path: projectPath, staged: false }),
        invoke<FileDiff[]>('git_diff', { path: projectPath, staged: true }),
      ]);

      const merged = mergeDiffs(unstaged, staged);
      setBranch(status.branch);
      setFiles(merged);
      setSelectedFile((current) => {
        if (current && merged.some((entry) => entry.file === current)) {
          return current;
        }
        return merged[0]?.file ?? null;
      });
    } catch (loadError) {
      console.error('Failed to load PR review state:', loadError);
      setError('Unable to load changed files for review.');
      setFiles([]);
      setSelectedFile(null);
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  useEffect(() => {
    void loadReviewState();
  }, [loadReviewState]);

  const selected = useMemo(
    () => files.find((entry) => entry.file === selectedFile) ?? null,
    [files, selectedFile],
  );

  const selectedReview = selected ? reviews[selected.file] : null;
  const totals = useMemo(() => {
    return files.reduce(
      (summary, file) => {
        summary.additions += file.additions;
        summary.deletions += file.deletions;
        return summary;
      },
      { additions: 0, deletions: 0 },
    );
  }, [files]);

  const openSelectedFile = useCallback(() => {
    if (!selected) {
      return;
    }
    const fullPath = normalizePath(joinPath(projectPath, selected.file));
    onOpenFile?.(fullPath);
  }, [onOpenFile, projectPath, selected]);

  const generateReview = useCallback(async () => {
    if (!selected) {
      return;
    }

    setIsReviewing(true);
    setError(null);

    try {
      const fullPath = normalizePath(joinPath(projectPath, selected.file));
      const result = await reviewDiff(
        selectedModel,
        buildUnifiedDiff(selected),
        fullPath,
        detectLanguage(selected.file),
      );

      setReviews((current) => ({
        ...current,
        [selected.file]: result,
      }));
    } catch (reviewError) {
      console.error('Failed to generate review:', reviewError);
      setError('Unable to generate AI review for the selected diff.');
    } finally {
      setIsReviewing(false);
    }
  }, [projectPath, selected, selectedModel]);

  const prepareFix = useCallback(
    async (commentId: string) => {
      if (!selected || !selectedReview) {
        return;
      }

      const comment = selectedReview.comments.find((entry) => entry.id === commentId);
      if (!comment) {
        return;
      }

      setIsGeneratingFix(true);
      setError(null);

      try {
        const fullPath = normalizePath(joinPath(projectPath, selected.file));
        const file = await readFile(fullPath);
        const prompt = [comment.title, comment.body, comment.suggestion]
          .filter(Boolean)
          .join('\n\n');

        const response = await invoke<string>('fix_code', {
          model: selectedModel,
          code: file.content,
          language: file.language || detectLanguage(selected.file),
          error: prompt,
        });

        const suggested = extractSuggestedCode(response);
        if (!suggested || suggested === file.content) {
          setError('AI fix generation did not produce a usable change.');
          return;
        }

        setFixPreview({
          commentId,
          original: file.content,
          suggested,
        });
      } catch (fixError) {
        console.error('Failed to prepare fix preview:', fixError);
        setError('Unable to generate a fix preview for this comment.');
      } finally {
        setIsGeneratingFix(false);
      }
    },
    [projectPath, selected, selectedModel, selectedReview],
  );

  const applyFix = useCallback(async () => {
    if (!selected || !fixPreview) {
      return;
    }

    setIsApplyingFix(true);
    setError(null);

    try {
      const fullPath = normalizePath(joinPath(projectPath, selected.file));
      await writeFile(fullPath, fixPreview.suggested);
      setFixPreview(null);
      setReviews((current) => {
        const next = { ...current };
        delete next[selected.file];
        return next;
      });
      await loadReviewState();
      onOpenFile?.(fullPath);
    } catch (applyError) {
      console.error('Failed to apply fix:', applyError);
      setError('Unable to apply the suggested fix to disk.');
    } finally {
      setIsApplyingFix(false);
    }
  }, [fixPreview, loadReviewState, onOpenFile, projectPath, selected]);

  return (
    <div className="h-full flex flex-col bg-[#0d1117] text-[#c9d1d9]">
      <div className="px-3 py-3 border-b border-[#30363d] space-y-3">
        <div className="flex items-center justify-between gap-2">
          <div>
            <div className="flex items-center gap-2 text-sm font-medium">
              <GitBranch size={14} className="text-[#8b949e]" />
              <span>PR Review</span>
            </div>
            <p className="text-[11px] text-[#8b949e] mt-1">
              Reviewing local branch changes on <span className="text-[#c9d1d9]">{branch}</span>
            </p>
          </div>
          <button
            onClick={() => void loadReviewState()}
            disabled={isLoading}
            className="p-1.5 rounded border border-[#30363d] text-[#8b949e] hover:text-[#c9d1d9] hover:bg-[#161b22] disabled:opacity-50"
            title="Refresh review state"
          >
            {isLoading ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
          </button>
        </div>

        <div className="grid grid-cols-3 gap-2 text-[11px]">
          <div className="rounded border border-[#30363d] bg-[#11161d] px-2 py-2">
            <div className="text-[#8b949e]">Files</div>
            <div className="text-sm font-semibold mt-1">{files.length}</div>
          </div>
          <div className="rounded border border-[#30363d] bg-[#11161d] px-2 py-2">
            <div className="text-[#8b949e]">Added</div>
            <div className="text-sm font-semibold mt-1 text-[#3fb950]">+{totals.additions}</div>
          </div>
          <div className="rounded border border-[#30363d] bg-[#11161d] px-2 py-2">
            <div className="text-[#8b949e]">Removed</div>
            <div className="text-sm font-semibold mt-1 text-[#f85149]">-{totals.deletions}</div>
          </div>
        </div>

        {error && (
          <div className="rounded border border-[#f85149]/30 bg-[#f85149]/10 px-2 py-2 text-[11px] text-[#ffb4ad]">
            {error}
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto">
        {files.length === 0 && !isLoading ? (
          <div className="h-full flex flex-col items-center justify-center text-[#8b949e] text-xs px-6 text-center">
            <Sparkles size={22} className="mb-2 opacity-50" />
            <p>No branch changes to review.</p>
            <p className="mt-1 text-[11px]">Make some edits or stage files to generate a reviewable diff.</p>
          </div>
        ) : (
          <div className="space-y-3 p-3">
            <section className="rounded border border-[#30363d] bg-[#11161d] overflow-hidden">
              <div className="px-3 py-2 border-b border-[#30363d] text-xs uppercase tracking-wide text-[#8b949e]">
                Changed Files
              </div>
              <div className="max-h-56 overflow-y-auto">
                {files.map((file) => {
                  const isSelected = file.file === selectedFile;
                  return (
                    <button
                      key={file.file}
                      onClick={() => setSelectedFile(file.file)}
                      className={`w-full text-left px-3 py-2 border-b border-[#1f242d] last:border-b-0 hover:bg-[#161b22] ${isSelected ? 'bg-[#161b22]' : ''}`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <div className="flex items-center gap-2 text-sm">
                            <FileCode2 size={13} className="text-[#8b949e] shrink-0" />
                            <span className="truncate">{file.file}</span>
                          </div>
                          <div className="mt-1 text-[11px] text-[#8b949e]">{sourceLabel(file.source)}</div>
                        </div>
                        <div className="shrink-0 text-[11px] text-right">
                          <div className="text-[#3fb950]">+{file.additions}</div>
                          <div className="text-[#f85149]">-{file.deletions}</div>
                        </div>
                      </div>
                    </button>
                  );
                })}
              </div>
            </section>

            {selected && (
              <>
                <section className="rounded border border-[#30363d] bg-[#11161d] overflow-hidden">
                  <div className="px-3 py-2 border-b border-[#30363d] flex items-center justify-between gap-2">
                    <div>
                      <div className="text-sm font-medium truncate">{selected.file}</div>
                      <div className="text-[11px] text-[#8b949e] mt-1">{selected.status} change</div>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={openSelectedFile}
                        className="px-2 py-1 rounded border border-[#30363d] text-[11px] text-[#c9d1d9] hover:bg-[#161b22]"
                      >
                        Open File
                      </button>
                      <button
                        onClick={() => void generateReview()}
                        disabled={isReviewing}
                        className="px-2 py-1 rounded bg-[#1f6feb] hover:bg-[#388bfd] text-[11px] text-white disabled:opacity-50 flex items-center gap-1"
                      >
                        {isReviewing ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} />}
                        Review Diff
                      </button>
                    </div>
                  </div>

                  <div className="max-h-72 overflow-y-auto font-mono text-[11px]">
                    {selected.hunks.map((hunk, hunkIndex) => (
                      <div key={`${hunk.header}-${hunkIndex}`}>
                        <div className="px-3 py-1 border-b border-[#1f242d] bg-[#161b22] text-[#8b949e]">
                          {hunk.header}
                        </div>
                        {hunk.lines.map((line, lineIndex) => {
                          const origin = line.origin || ' ';
                          const tone = origin === '+'
                            ? 'bg-[#2ea04320]'
                            : origin === '-'
                              ? 'bg-[#f8514920]'
                              : '';

                          return (
                            <div key={`${hunkIndex}-${lineIndex}`} className={`flex ${tone}`}>
                              <span className="w-10 text-right pr-2 text-[#6e7681] select-none">{line.old_lineno ?? ''}</span>
                              <span className="w-10 text-right pr-2 text-[#6e7681] select-none">{line.new_lineno ?? ''}</span>
                              <span className="w-4 text-center text-[#8b949e] select-none">{origin}</span>
                              <span className="flex-1 px-2 whitespace-pre-wrap break-words">{line.content}</span>
                            </div>
                          );
                        })}
                      </div>
                    ))}
                  </div>
                </section>

                {selectedReview && (
                  <section className="rounded border border-[#30363d] bg-[#11161d] overflow-hidden">
                    <div className="px-3 py-2 border-b border-[#30363d] flex items-center justify-between gap-2">
                      <div>
                        <div className="text-sm font-medium">AI Review Summary</div>
                        <div className="text-[11px] text-[#8b949e] mt-1">Per-file review summary, checklist, and comments</div>
                      </div>
                      <div className={`px-2 py-1 rounded border text-[11px] uppercase tracking-wide ${riskTone(selectedReview.risk)}`}>
                        {selectedReview.risk} risk
                      </div>
                    </div>

                    <div className="p-3 space-y-3 text-sm">
                      <p className="text-[#c9d1d9] leading-6">{selectedReview.summary}</p>

                      <div className="space-y-2">
                        <div className="text-[11px] uppercase tracking-wide text-[#8b949e]">Review Checklist</div>
                        {selectedReview.checklist.map((item, index) => (
                          <div key={`${item.label}-${index}`} className="flex items-start gap-2 text-[12px]">
                            {item.checked ? (
                              <CheckCircle2 size={13} className="mt-0.5 text-[#3fb950] shrink-0" />
                            ) : (
                              <AlertTriangle size={13} className="mt-0.5 text-[#d29922] shrink-0" />
                            )}
                            <div>
                              <div className="text-[#c9d1d9]">{item.label}</div>
                              {item.detail && <div className="text-[#8b949e] mt-0.5">{item.detail}</div>}
                            </div>
                          </div>
                        ))}
                      </div>

                      <div className="space-y-2">
                        <div className="text-[11px] uppercase tracking-wide text-[#8b949e]">Review Comments</div>
                        {selectedReview.comments.map((comment) => (
                          <div key={comment.id} className={`rounded border px-3 py-3 space-y-2 ${commentTone(comment.severity)}`}>
                            <div className="flex items-start justify-between gap-2">
                              <div>
                                <div className="text-sm font-medium text-[#c9d1d9]">{comment.title}</div>
                                <div className="text-[11px] text-[#8b949e] mt-1">
                                  {comment.line ? `Line ${comment.line}` : 'File-level comment'}
                                </div>
                              </div>
                              <div className="flex items-center gap-1 text-[11px] uppercase tracking-wide text-[#8b949e]">
                                <ShieldAlert size={12} />
                                {comment.severity}
                              </div>
                            </div>
                            <p className="text-[12px] text-[#c9d1d9] leading-5">{comment.body}</p>
                            {comment.suggestion && (
                              <div className="rounded border border-[#30363d] bg-[#0d1117] px-2 py-2 text-[12px] text-[#8b949e]">
                                {comment.suggestion}
                              </div>
                            )}
                            <div className="flex justify-end">
                              <button
                                onClick={() => void prepareFix(comment.id)}
                                disabled={isGeneratingFix || isApplyingFix}
                                className="px-2 py-1 rounded border border-[#30363d] text-[11px] text-[#c9d1d9] hover:bg-[#161b22] disabled:opacity-50 flex items-center gap-1"
                              >
                                {isGeneratingFix ? <Loader2 size={12} className="animate-spin" /> : <Wand2 size={12} />}
                                Generate Fix Preview
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  </section>
                )}

                {fixPreview && (
                  <section className="rounded border border-[#30363d] bg-[#11161d] overflow-hidden">
                    <div className="px-3 py-2 border-b border-[#30363d] flex items-center justify-between gap-2">
                      <div>
                        <div className="text-sm font-medium">Fix Preview</div>
                        <div className="text-[11px] text-[#8b949e] mt-1">Review the generated file replacement before applying it.</div>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => setFixPreview(null)}
                          className="px-2 py-1 rounded border border-[#30363d] text-[11px] text-[#c9d1d9] hover:bg-[#161b22]"
                        >
                          Cancel
                        </button>
                        <button
                          onClick={() => void applyFix()}
                          disabled={isApplyingFix}
                          className="px-2 py-1 rounded bg-[#238636] hover:bg-[#2ea043] text-[11px] text-white disabled:opacity-50 flex items-center gap-1"
                        >
                          {isApplyingFix ? <Loader2 size={12} className="animate-spin" /> : <CheckCircle2 size={12} />}
                          Apply Fix
                        </button>
                      </div>
                    </div>
                    <div className="grid grid-cols-1 gap-px bg-[#30363d] md:grid-cols-2">
                      <div className="bg-[#0d1117] p-3">
                        <div className="text-[11px] uppercase tracking-wide text-[#8b949e] mb-2">Current File</div>
                        <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-words font-mono text-[11px] text-[#c9d1d9]">{fixPreview.original}</pre>
                      </div>
                      <div className="bg-[#0d1117] p-3">
                        <div className="text-[11px] uppercase tracking-wide text-[#8b949e] mb-2">Suggested File</div>
                        <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-words font-mono text-[11px] text-[#c9d1d9]">{fixPreview.suggested}</pre>
                      </div>
                    </div>
                  </section>
                )}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export default PRReviewPanel;
