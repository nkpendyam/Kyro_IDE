---
name: typescript-react-expert
description: Use this skill for all TypeScript and React code — components, Zustand stores, Tauri frontend bindings, shadcn/ui, Tailwind CSS, Monaco editor integration, and Next.js. Triggers on: .tsx, .ts files, "component", "hook", "zustand", "tailwind", "shadcn", "monaco".
---

# TypeScript + React Expert Skill

## Core rules for Kyro IDE frontend

### NEVER use `any`
```typescript
// ❌ WRONG
const data: any = await invoke('get_data');

// ✅ CORRECT
interface DataResponse {
  items: string[];
  count: number;
}
const data = await invoke<DataResponse>('get_data');
```

### Standard Tauri command binding pattern
```typescript
// src/lib/tauri-commands.ts
import { invoke } from '@tauri-apps/api/core';

// Always typed, always async, always handle errors
export const gitStage = async (
  repoPath: string,
  filePath: string
): Promise<void> => {
  return invoke<void>('git_stage', { repoPath, filePath });
};

// Usage in component
const handleStage = async (file: string) => {
  try {
    await gitStage(projectPath, file);
    toast.success('File staged');
  } catch (error) {
    toast.error(`Failed to stage: ${error}`);
  }
};
```

### Standard Zustand store pattern
```typescript
// src/store/exampleStore.ts
import { create } from 'zustand';

interface ExampleState {
  items: string[];
  isLoading: boolean;
  error: string | null;
  // Actions
  fetchItems: () => Promise<void>;
  addItem: (item: string) => void;
}

export const useExampleStore = create<ExampleState>((set) => ({
  items: [],
  isLoading: false,
  error: null,

  fetchItems: async () => {
    set({ isLoading: true, error: null });
    try {
      const data = await invoke<string[]>('get_items');
      set({ items: data, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  addItem: (item) => set((state) => ({
    items: [...state.items, item]
  })),
}));
```

### Standard React component pattern
```typescript
// src/components/panels/ExamplePanel.tsx
import { useEffect } from 'react';
import { useExampleStore } from '@/store/exampleStore';

interface ExamplePanelProps {
  className?: string;
}

export function ExamplePanel({ className }: ExamplePanelProps) {
  const { items, isLoading, error, fetchItems } = useExampleStore();

  useEffect(() => {
    fetchItems();
  }, [fetchItems]);

  if (isLoading) return <div className="p-4">Loading...</div>;
  if (error) return <div className="p-4 text-destructive">{error}</div>;

  return (
    <div className={className}>
      {items.map((item, i) => (
        <div key={i}>{item}</div>
      ))}
    </div>
  );
}
```

### Tauri event listener pattern
```typescript
// Listening for events from Rust backend
import { listen } from '@tauri-apps/api/event';

useEffect(() => {
  const unlisten = listen<AgentUpdate>('agent_update', (event) => {
    setAgentProgress(event.payload);
  });

  // CRITICAL: always cleanup to prevent memory leaks
  return () => {
    unlisten.then(fn => fn());
  };
}, []);
```

## Performance patterns

### Lazy loading Monaco (fixes cold start)
```typescript
import { lazy, Suspense } from 'react';

const MonacoEditor = lazy(() => import('@monaco-editor/react'));

export function Editor() {
  return (
    <Suspense fallback={<EditorSkeleton />}>
      <MonacoEditor />
    </Suspense>
  );
}
```

### Virtualizing large lists (fixes file tree freeze)
```typescript
import { FixedSizeList } from 'react-window';

export function FileTree({ files }: { files: string[] }) {
  return (
    <FixedSizeList
      height={600}
      itemCount={files.length}
      itemSize={24}
      width="100%"
    >
      {({ index, style }) => (
        <div style={style}>{files[index]}</div>
      )}
    </FixedSizeList>
  );
}
```

### Debouncing AI completions
```typescript
import { useCallback, useRef } from 'react';

export function useDebounce<T extends (...args: unknown[]) => unknown>(
  fn: T,
  delay: number
): T {
  const timerRef = useRef<ReturnType<typeof setTimeout>>();
  const abortRef = useRef<AbortController>();

  return useCallback((...args: Parameters<T>) => {
    clearTimeout(timerRef.current);
    abortRef.current?.abort();
    abortRef.current = new AbortController();

    timerRef.current = setTimeout(() => {
      fn(...args);
    }, delay);
  }, [fn, delay]) as T;
}
```
