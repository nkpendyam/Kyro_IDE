import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';

import { UpdaterProvider, useUpdater } from '@/components/updater/UpdaterProvider';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

function TestHarness() {
  const { checkForUpdates } = useUpdater();

  return (
    <button type="button" onClick={() => void checkForUpdates(true)}>
      Open updater
    </button>
  );
}

describe('UpdaterProvider', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    let progressChecks = 0;
    mockInvoke.mockImplementation(async (command: string) => {
      if (command === 'check_for_updates') {
        return {
          version: '0.3.0',
          currentVersion: '0.2.0',
          releaseDate: '2026-03-15T00:00:00Z',
          releaseNotes: 'Security fixes and updater hardening.',
          channel: 'stable',
          sizeMb: 42.5,
          mandatory: true,
          target: 'windows-x86_64',
        };
      }
      if (command === 'download_update') {
        return {
          downloadedBytes: 1024,
          totalBytes: 1024,
          percentage: 100,
          completed: true,
        };
      }
      if (command === 'get_download_progress') {
        progressChecks += 1;
        return {
          downloadedBytes: progressChecks > 1 ? 1024 : 256,
          totalBytes: 1024,
          percentage: progressChecks > 1 ? 100 : 25,
          completed: progressChecks > 1,
        };
      }
      if (command === 'install_update') {
        return undefined;
      }
      throw new Error(`Unexpected command: ${command}`);
    });
  });

  it('shows update metadata and runs the restart flow', async () => {
    render(
      <UpdaterProvider>
        <TestHarness />
      </UpdaterProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: /open updater/i }));

    await waitFor(() => {
      expect(screen.getByText('Update Ready for Kyro IDE')).toBeInTheDocument();
      expect(screen.getByText('Security fixes and updater hardening.')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /download and restart/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('download_update', undefined);
      expect(mockInvoke).toHaveBeenCalledWith('install_update', undefined);
    });
  });
});
