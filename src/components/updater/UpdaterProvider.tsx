'use client';

import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Check, Download, Loader2, RefreshCw, X } from 'lucide-react';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Progress } from '@/components/ui/progress';

export interface UpdaterInfo {
  version: string;
  currentVersion: string;
  releaseDate?: string | null;
  releaseNotes: string;
  channel: string;
  sizeMb?: number | null;
  mandatory: boolean;
  target: string;
}

export interface UpdaterProgress {
  downloadedBytes: number;
  totalBytes?: number | null;
  percentage: number;
  completed: boolean;
}

interface UpdaterContextValue {
  update: UpdaterInfo | null;
  progress: UpdaterProgress | null;
  checking: boolean;
  downloading: boolean;
  installing: boolean;
  error: string | null;
  isDialogOpen: boolean;
  checkForUpdates: (openDialog?: boolean) => Promise<UpdaterInfo | null>;
  downloadAndInstall: () => Promise<void>;
  dismiss: () => void;
}

const UpdaterContext = createContext<UpdaterContextValue | null>(null);

async function invokeCommand<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, payload);
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} B`;
}

export function UpdaterProvider({ children }: { children: React.ReactNode }) {
  const [update, setUpdate] = useState<UpdaterInfo | null>(null);
  const [progress, setProgress] = useState<UpdaterProgress | null>(null);
  const [checking, setChecking] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const pollTimerRef = useRef<number | null>(null);

  const stopPolling = useCallback(() => {
    if (pollTimerRef.current !== null) {
      window.clearTimeout(pollTimerRef.current);
      pollTimerRef.current = null;
    }
  }, []);

  const pollProgress = useCallback(async () => {
    try {
      const nextProgress = await invokeCommand<UpdaterProgress>('get_download_progress');
      setProgress(nextProgress);
      if (!nextProgress.completed) {
        pollTimerRef.current = window.setTimeout(() => {
          void pollProgress();
        }, 250);
      }
    } catch (pollError) {
      setError(String(pollError));
      setDownloading(false);
      stopPolling();
    }
  }, [stopPolling]);

  useEffect(() => stopPolling, [stopPolling]);

  const checkForUpdates = useCallback(async (openDialog = false) => {
    setChecking(true);
    setError(null);

    try {
      const nextUpdate = await invokeCommand<UpdaterInfo | null>('check_for_updates');
      setUpdate(nextUpdate);
      if (nextUpdate && openDialog) {
        setIsDialogOpen(true);
      }
      return nextUpdate;
    } catch (checkError) {
      const message = String(checkError);
      setError(message);
      if (openDialog) {
        setIsDialogOpen(true);
      }
      return null;
    } finally {
      setChecking(false);
    }
  }, []);

  const downloadAndInstall = useCallback(async () => {
    setDownloading(true);
    setInstalling(false);
    setError(null);
    setProgress({ downloadedBytes: 0, totalBytes: null, percentage: 0, completed: false });

    try {
      void pollProgress();
      const finalProgress = await invokeCommand<UpdaterProgress>('download_update');
      setProgress(finalProgress);
      setDownloading(false);
      setInstalling(true);
      await invokeCommand<void>('install_update');
    } catch (installError) {
      setError(String(installError));
      setDownloading(false);
      setInstalling(false);
      stopPolling();
    }
  }, [pollProgress, stopPolling]);

  const dismiss = useCallback(() => {
    setIsDialogOpen(false);
    setError(null);
  }, []);

  useEffect(() => {
    void checkForUpdates(false);
  }, [checkForUpdates]);

  const value = useMemo<UpdaterContextValue>(() => ({
    update,
    progress,
    checking,
    downloading,
    installing,
    error,
    isDialogOpen,
    checkForUpdates,
    downloadAndInstall,
    dismiss,
  }), [checkForUpdates, dismiss, downloadAndInstall, downloading, error, installing, isDialogOpen, progress, checking, update]);

  return (
    <UpdaterContext.Provider value={value}>
      {children}
      <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
        <DialogContent className="max-w-xl border-[#30363d] bg-[#0d1117] text-[#c9d1d9]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-[#f0f6fc]">
              <Download className="size-4 text-[#58a6ff]" />
              Update Ready for Kyro IDE
            </DialogTitle>
            <DialogDescription className="text-[#8b949e]">
              Download, verify, and restart into the latest signed desktop release.
            </DialogDescription>
          </DialogHeader>

          {update ? (
            <div className="space-y-4">
              <div className="rounded-lg border border-[#30363d] bg-[#11161d] p-4 text-sm">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="font-medium text-[#f0f6fc]">v{update.version}</div>
                    <div className="text-[#8b949e]">
                      Current v{update.currentVersion} · {update.channel} · {update.target}
                    </div>
                  </div>
                  {update.sizeMb ? (
                    <div className="text-xs text-[#8b949e]">{update.sizeMb.toFixed(1)} MB</div>
                  ) : null}
                </div>
                {update.releaseDate ? (
                  <div className="mt-2 text-xs text-[#8b949e]">Published {update.releaseDate}</div>
                ) : null}
                <div className="mt-3 max-h-40 overflow-y-auto whitespace-pre-wrap rounded-md bg-[#0d1117] p-3 text-xs leading-5 text-[#c9d1d9]">
                  {update.releaseNotes || 'No release notes were included with this update.'}
                </div>
              </div>

              {progress ? (
                <div className="space-y-2 rounded-lg border border-[#30363d] bg-[#11161d] p-4">
                  <div className="flex items-center justify-between text-xs text-[#8b949e]">
                    <span>
                      {formatBytes(progress.downloadedBytes)}
                      {progress.totalBytes ? ` / ${formatBytes(progress.totalBytes)}` : ''}
                    </span>
                    <span>{Math.round(progress.percentage)}%</span>
                  </div>
                  <Progress value={progress.percentage} className="bg-[#21262d] [&_[data-slot=progress-indicator]]:bg-[#2ea043]" />
                </div>
              ) : null}

              {error ? (
                <div className="rounded-lg border border-[#f85149]/40 bg-[#f85149]/10 p-3 text-sm text-[#ffb4ad]">
                  {error}
                </div>
              ) : null}
            </div>
          ) : (
            <div className="rounded-lg border border-[#30363d] bg-[#11161d] p-4 text-sm text-[#8b949e]">
              {checking ? 'Checking for updates...' : error || 'No update is currently available.'}
            </div>
          )}

          <DialogFooter>
            <button
              type="button"
              onClick={dismiss}
              className="inline-flex items-center gap-2 rounded-md border border-[#30363d] px-4 py-2 text-sm text-[#c9d1d9] hover:bg-[#161b22]"
            >
              <X className="size-4" />
              Later
            </button>
            <button
              type="button"
              onClick={() => void checkForUpdates(true)}
              disabled={checking || downloading || installing}
              className="inline-flex items-center gap-2 rounded-md border border-[#30363d] px-4 py-2 text-sm text-[#c9d1d9] hover:bg-[#161b22] disabled:opacity-60"
            >
              {checking ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
              Check Again
            </button>
            <button
              type="button"
              onClick={() => void downloadAndInstall()}
              disabled={!update || checking || downloading || installing}
              className="inline-flex items-center gap-2 rounded-md bg-[#238636] px-4 py-2 text-sm font-medium text-white hover:bg-[#2ea043] disabled:opacity-60"
            >
              {downloading || installing ? <Loader2 className="size-4 animate-spin" /> : <Check className="size-4" />}
              {installing ? 'Restarting…' : downloading ? 'Downloading…' : 'Download and Restart'}
            </button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </UpdaterContext.Provider>
  );
}

export function useUpdater() {
  const context = useContext(UpdaterContext);
  if (!context) {
    throw new Error('useUpdater must be used within an UpdaterProvider');
  }
  return context;
}
