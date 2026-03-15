import { invoke } from '@tauri-apps/api/core';

export const gitStage = (repoPath: string, filePath: string) =>
  invoke<void>('git_stage', { repoPath, filePath });

export const gitUnstage = (repoPath: string, filePath: string) =>
  invoke<void>('git_unstage', { repoPath, filePath });

export const gitStageAll = (repoPath: string) =>
  invoke<void>('git_stage_all', { repoPath });

export const gitUnstageAll = (repoPath: string) =>
  invoke<void>('git_unstage_all', { repoPath });

export const gitDiscard = (repoPath: string, filePath: string) =>
  invoke<void>('git_discard', { repoPath, filePath });

export const gitStageHunk = (repoPath: string, filePath: string, hunkIndex: number) =>
  invoke<void>('git_stage_hunk', { repoPath, filePath, hunkIndex });

export interface BroadcastCursorPayload {
  line: number;
  column: number;
  file?: string;
}

export const broadcastCursor = (roomId: string, cursor: BroadcastCursorPayload) =>
  invoke<void>('broadcast_cursor', { roomId, cursor });

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

export const reviewDiff = (
  model: string,
  diff: string,
  filePath: string,
  language: string,
) => invoke<DiffReviewResult>('review_diff', { model, diff, filePath, language });
