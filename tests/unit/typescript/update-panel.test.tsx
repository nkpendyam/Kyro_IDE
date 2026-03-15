import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';

import { UpdatePanel } from '@/components/update/UpdatePanel';

const mockInvoke = vi.fn();
const mockUpdaterContext = {
  update: {
    version: '0.3.0',
    currentVersion: '0.2.0',
    releaseDate: '2026-03-15T00:00:00Z',
    releaseNotes: 'Signed update available.',
    channel: 'stable',
    sizeMb: 33.2,
    mandatory: false,
    target: 'windows-x86_64',
  },
  progress: null,
  checking: false,
  downloading: false,
  installing: false,
  error: null,
  isDialogOpen: false,
  checkForUpdates: vi.fn(),
  downloadAndInstall: vi.fn(),
  dismiss: vi.fn(),
};

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@/components/updater/UpdaterProvider', () => ({
  useUpdater: () => mockUpdaterContext,
}));

describe('UpdatePanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation(async (command: string) => {
      if (command === 'get_update_channel') {
        return 'stable';
      }
      if (command === 'is_auto_update_enabled') {
        return true;
      }
      if (command === 'skip_update' || command === 'set_update_channel' || command === 'set_auto_update') {
        return undefined;
      }
      throw new Error(`Unexpected command: ${command}`);
    });
  });

  it('renders release metadata and can skip optional updates', async () => {
    render(<UpdatePanel />);

    await waitFor(() => {
      expect(screen.getByText('Update Available: v0.3.0')).toBeInTheDocument();
      expect(screen.getByText('Signed update available.')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /skip/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('skip_update', { version: '0.3.0' });
      expect(mockUpdaterContext.checkForUpdates).toHaveBeenCalledWith(false);
    });
  });
});
