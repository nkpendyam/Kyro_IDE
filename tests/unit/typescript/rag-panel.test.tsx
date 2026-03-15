import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';

import { RagPanel } from '@/components/rag/RagPanel';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('RagPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation(async (command: string) => {
      if (command === 'get_rag_status') {
        return {
          indexed_files: 12,
          total_chunks: 44,
          index_size_mb: 3.4,
          last_indexed: '2026-03-15T00:00:00Z',
          is_indexing: false,
        };
      }

      if (command === 'graph_enhanced_semantic_search') {
        return [
          {
            file_path: 'src/parser.rs',
            content: 'fn parse() {}',
            score: 0.91,
            line_start: 10,
            line_end: 18,
            context: 'Dependency graph neighbor discovered 1 hops away',
            source: 'graphNeighbor',
            graph_score: 0.72,
            graph_distance: 1,
            neighbors: ['src/app.rs'],
          },
        ];
      }

      if (command === 'clear_rag_index') {
        return undefined;
      }

      throw new Error(`Unexpected command: ${command}`);
    });
  });

  it('uses graph-enhanced search and renders graph metadata', async () => {
    render(<RagPanel />);

    fireEvent.change(screen.getByPlaceholderText('Semantic search...'), {
      target: { value: 'parser graph' },
    });

    await act(async () => {
      fireEvent.click(screen.getByRole('radio', { name: /drift/i }));
    });

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /^search$/i }));
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('graph_enhanced_semantic_search', {
        request: { query: 'parser graph', maxResults: 10, graphMode: 'drift' },
      });
      expect(screen.getByText('src/parser.rs')).toBeInTheDocument();
      expect(screen.getByText(/Graph 72%/i)).toBeInTheDocument();
      expect(screen.getByText(/Dependency graph neighbor discovered 1 hops away/i)).toBeInTheDocument();
    });
  });
});
