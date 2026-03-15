'use client';

import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ArrowUp, Check, Download, Info, RefreshCw } from 'lucide-react';

import { Progress } from '@/components/ui/progress';
import { useUpdater } from '@/components/updater/UpdaterProvider';

export function UpdatePanel() {
  const {
    update,
    progress,
    checking,
    downloading,
    installing,
    error,
    checkForUpdates,
    downloadAndInstall,
  } = useUpdater();
  const [channel, setChannel] = useState('stable');
  const [autoUpdate, setAutoUpdate] = useState(true);
  const [settingsError, setSettingsError] = useState<string | null>(null);

  useEffect(() => {
    async function loadSettings() {
      try {
        const [nextChannel, nextAuto] = await Promise.all([
          invoke<string>('get_update_channel'),
          invoke<boolean>('is_auto_update_enabled'),
        ]);
        setChannel(nextChannel);
        setAutoUpdate(nextAuto);
      } catch (loadError) {
        setSettingsError(String(loadError));
      }
    }

    void loadSettings();
  }, []);

  const activeError = error ?? settingsError;

  return (
    <div className="flex h-full flex-col bg-[#0d1117] p-4 text-[#c9d1d9]">
      <div className="mb-4 flex items-center justify-between">
        <h3 className="font-medium">Updates</h3>
        <button
          onClick={() => void checkForUpdates(true)}
          disabled={checking || downloading || installing}
          className="flex items-center gap-2 rounded bg-[#21262d] px-3 py-1.5 text-sm text-[#c9d1d9] hover:bg-[#30363d] disabled:opacity-50"
        >
          <RefreshCw size={16} className={checking ? 'animate-spin' : ''} />
          Check for Updates
        </button>
      </div>

      {activeError ? (
        <div className="mb-4 rounded border border-[#f85149]/40 bg-[#f85149]/10 px-4 py-2 text-sm text-[#ffb4ad]">
          {activeError}
        </div>
      ) : null}

      {update ? (
        <div className="mb-4 rounded border border-[#30363d] bg-[#161b22] p-4">
          <div className="flex items-start gap-3">
            <div className="rounded bg-[#238636] p-2">
              <ArrowUp size={20} className="text-white" />
            </div>
            <div className="flex-1 space-y-3">
              <div>
                <h4 className="mb-1 font-medium text-[#f0f6fc]">Update Available: v{update.version}</h4>
                <p className="text-sm text-[#8b949e]">
                  Current: v{update.currentVersion}
                  {update.sizeMb ? ` • Size: ${update.sizeMb.toFixed(1)} MB` : ''}
                  {update.releaseDate ? ` • Published ${update.releaseDate}` : ''}
                </p>
              </div>

              <div className="max-h-40 overflow-y-auto rounded bg-[#0d1117] p-3 text-sm text-[#8b949e]">
                {update.releaseNotes || 'No release notes were provided for this release.'}
              </div>

              {progress ? (
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-sm text-[#8b949e]">
                    <span>
                      {progress.downloadedBytes.toLocaleString()} bytes
                      {progress.totalBytes ? ` / ${progress.totalBytes.toLocaleString()} bytes` : ''}
                    </span>
                    <span>{Math.round(progress.percentage)}%</span>
                  </div>
                  <Progress value={progress.percentage} className="bg-[#21262d] [&_[data-slot=progress-indicator]]:bg-[#238636]" />
                </div>
              ) : null}

              <div className="flex gap-2">
                <button
                  onClick={() => void downloadAndInstall()}
                  disabled={checking || downloading || installing}
                  className="flex items-center gap-2 rounded bg-[#238636] px-4 py-2 text-sm text-white hover:bg-[#2ea043] disabled:opacity-50"
                >
                  {downloading || installing ? <RefreshCw size={16} className="animate-spin" /> : <Download size={16} />}
                  {installing ? 'Restarting…' : downloading ? 'Downloading…' : 'Download & Restart'}
                </button>
                {!update.mandatory ? (
                  <button
                    onClick={async () => {
                      await invoke('skip_update', { version: update.version });
                      await checkForUpdates(false);
                    }}
                    className="rounded bg-[#21262d] px-4 py-2 text-sm text-[#c9d1d9] hover:bg-[#30363d]"
                  >
                    Skip
                  </button>
                ) : null}
              </div>
            </div>
          </div>
        </div>
      ) : !checking ? (
        <div className="flex flex-col items-center justify-center py-8 text-[#8b949e]">
          <Check size={48} className="mb-4 text-[#3fb950]" />
          <p className="text-[#c9d1d9]">You&apos;re up to date.</p>
          <p className="text-sm">Signed releases will appear here automatically.</p>
        </div>
      ) : null}

      <div className="mt-auto border-t border-[#30363d] pt-4">
        <h4 className="mb-3 text-sm font-medium text-[#c9d1d9]">Update Settings</h4>

        <div className="mb-3 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Info size={14} className="text-[#8b949e]" />
            <span className="text-sm text-[#8b949e]">Update Channel</span>
          </div>
          <select
            value={channel}
            onChange={async (event) => {
              const nextChannel = event.target.value;
              await invoke('set_update_channel', { channel: nextChannel });
              setChannel(nextChannel);
              await checkForUpdates(false);
            }}
            className="rounded border border-[#30363d] bg-[#161b22] px-3 py-1 text-sm text-[#c9d1d9]"
          >
            <option value="stable">Stable</option>
            <option value="beta">Beta</option>
            <option value="nightly">Nightly</option>
          </select>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm text-[#8b949e]">Auto Update</span>
          <button
            onClick={async () => {
              const nextValue = !autoUpdate;
              await invoke('set_auto_update', { enabled: nextValue });
              setAutoUpdate(nextValue);
            }}
            className={`relative h-6 w-12 rounded-full ${autoUpdate ? 'bg-[#238636]' : 'bg-[#21262d]'}`}
          >
            <div
              className={`absolute top-1 h-4 w-4 rounded-full bg-white transition-transform ${autoUpdate ? 'translate-x-7' : 'translate-x-1'}`}
            />
          </button>
        </div>
      </div>
    </div>
  );
}
