import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';

import { PRReviewPanel } from '@/components/git/PRReviewPanel';

const mockInvoke = vi.fn();
const mockReadFile = vi.fn();
const mockWriteFile = vi.fn();
const mockReviewDiff = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@/lib/fileOperations', () => ({
  readFile: (...args: unknown[]) => mockReadFile(...args),
  writeFile: (...args: unknown[]) => mockWriteFile(...args),
  detectLanguage: () => 'typescript',
  joinPath: (...parts: string[]) => parts.join('/').replace(/\/+/g, '/'),
  normalizePath: (value: string) => value.replace(/\\/g, '/'),
}));

vi.mock('@/lib/tauri-commands', () => ({
  reviewDiff: (...args: unknown[]) => mockReviewDiff(...args),
}));

vi.mock('@/store/kyroStore', () => ({
  useKyroStore: (selector: (state: { selectedModel: string }) => unknown) =>
    selector({ selectedModel: 'codellama:7b' }),
}));

describe('PRReviewPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    mockInvoke.mockImplementation(async (command: string, payload?: { staged?: boolean }) => {
      if (command === 'git_status') {
        return { branch: 'feature/pr-review' };
      }
      if (command === 'git_diff') {
        if (payload?.staged) {
          return [];
        }
        return [
          {
            file: 'src/example.ts',
            status: 'modified',
            additions: 2,
            deletions: 1,
            hunks: [
              {
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 2,
                header: '@@ -1,1 +1,2 @@',
                lines: [
                  { old_lineno: 1, new_lineno: null, origin: '-', content: 'const oldValue = true;' },
                  { old_lineno: null, new_lineno: 1, origin: '+', content: 'const nextValue = compute();' },
                  { old_lineno: null, new_lineno: 2, origin: '+', content: 'return nextValue;' },
                ],
              },
            ],
          },
        ];
      }
      if (command === 'fix_code') {
        return '```ts\nexport const updated = true;\n```';
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    mockReviewDiff.mockResolvedValue({
      summary: 'The patch simplifies a branch but should preserve a null guard.',
      risk: 'medium',
      checklist: [
        { label: 'Behavior is reviewed', checked: true, detail: 'The control flow is simpler now.' },
        { label: 'Regression coverage considered', checked: false, detail: 'Add one focused test.' },
      ],
      comments: [
        {
          id: 'c-1',
          severity: 'warning',
          title: 'Preserve the null guard',
          body: 'This change removes a defensive branch that previously handled missing input.',
          line: 1,
          suggestion: 'Restore the guard before computing the next value.',
        },
      ],
    });

    mockReadFile.mockResolvedValue({
      path: '/repo/src/example.ts',
      language: 'typescript',
      content: 'export const current = false;\n',
    });
    mockWriteFile.mockResolvedValue(undefined);
  });

  it('loads changed files and renders summary stats', async () => {
    render(<PRReviewPanel projectPath="/repo" />);

    await waitFor(() => {
      expect(screen.getAllByText('src/example.ts').length).toBeGreaterThan(0);
    });

    expect(screen.getByText('feature/pr-review')).toBeInTheDocument();
    expect(screen.getAllByText('+2').length).toBeGreaterThan(0);
    expect(screen.getAllByText('-1').length).toBeGreaterThan(0);
  });

  it('generates a review and applies a suggested fix', async () => {
    const onOpenFile = vi.fn();
    render(<PRReviewPanel projectPath="/repo" onOpenFile={onOpenFile} />);

    await waitFor(() => {
      expect(screen.getAllByText('src/example.ts').length).toBeGreaterThan(0);
    });

    fireEvent.click(screen.getByRole('button', { name: /review diff/i }));

    await waitFor(() => {
      expect(screen.getByText('AI Review Summary')).toBeInTheDocument();
      expect(screen.getByText('Preserve the null guard')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /generate fix preview/i }));

    await waitFor(() => {
      expect(screen.getByText('Fix Preview')).toBeInTheDocument();
      expect(screen.getByText('Suggested File')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /apply fix/i }));

    await waitFor(() => {
      expect(mockWriteFile).toHaveBeenCalledWith('/repo/src/example.ts', 'export const updated = true;');
    });

    expect(onOpenFile).toHaveBeenCalledWith('/repo/src/example.ts');
  });
});
