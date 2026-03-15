import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';

import { SymbolOutline } from '@/components/sidebar/SymbolOutline';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@/store/kyroStore', () => ({
  useKyroStore: () => ({
    activeFileIndex: 0,
    openFiles: [
      {
        language: 'rust',
        content: 'pub struct App {\n}\n\nfn boot() {}',
      },
    ],
  }),
}));

describe('SymbolOutline', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue({
      language: 'rust',
      symbols: [
        {
          name: 'App',
          kind: 'Struct',
          start_line: 1,
          start_col: 5,
          end_line: 2,
          end_col: 1,
          documentation: '/// Application root',
        },
        {
          name: 'boot',
          kind: 'Function',
          start_line: 4,
          start_col: 1,
          end_line: 4,
          end_col: 12,
          documentation: null,
        },
      ],
    });
  });

  it('renders extracted tree-sitter symbols for the current file', async () => {
    render(<SymbolOutline />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('extract_symbols', {
        language: 'rust',
        code: 'pub struct App {\n}\n\nfn boot() {}',
      });
      expect(screen.getByText('App')).toBeInTheDocument();
      expect(screen.getByText('boot')).toBeInTheDocument();
      expect(screen.getByText(/2 symbols/i)).toBeInTheDocument();
    });
  });
});
