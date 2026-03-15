declare module '@tauri-apps/plugin-dialog' {
  interface OpenDialogOptions {
    directory?: boolean;
    multiple?: boolean;
    defaultPath?: string;
    filters?: Array<{ name: string; extensions: string[] }>;
    title?: string;
  }

  export function open(options?: OpenDialogOptions): Promise<string | string[] | null>;
  export function save(options?: OpenDialogOptions): Promise<string | null>;
  export function message(message: string, options?: { title?: string; type?: 'info' | 'warning' | 'error' }): Promise<void>;
  export function ask(message: string, options?: { title?: string; type?: 'info' | 'warning' | 'error' }): Promise<boolean>;
  export function confirm(message: string, options?: { title?: string; type?: 'info' | 'warning' | 'error' }): Promise<boolean>;
}

declare global {
  interface Window {
    __TAURI__?: {
      core: {
        invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
      };
    };
  }
}

export {};
