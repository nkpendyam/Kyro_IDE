'use client';

import React, { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// User cursor from collaboration
export interface RemoteCursor {
  userId: string;
  userName: string;
  color: string;
  line: number;
  column: number;
  selection?: {
    startLine: number;
    startColumn: number;
    endLine: number;
    endColumn: number;
  };
}

// Props for cursor overlay
interface EditorPresenceProps {
  // Editor container ref for positioning
  editorContainerRef: React.RefObject<HTMLDivElement | null>;
  // Monaco editor instance
  editor?: unknown;
  // Current room ID
  roomId?: string;
  // Current user ID
  currentUserId?: string;
  currentUserName?: string;
}

// User colors for cursors
const USER_COLORS = [
  '#FF6B6B', // Red
  '#4ECDC4', // Teal
  '#45B7D1', // Blue
  '#96CEB4', // Green
  '#FFEAA7', // Yellow
  '#DDA0DD', // Plum
  '#98D8C8', // Mint
  '#F7DC6F', // Gold
  '#BB8FCE', // Purple
  '#85C1E9', // Sky
];

// Get color for user based on ID
function getUserColor(userId: string): string {
  let hash = 0;
  for (let i = 0; i < userId.length; i++) {
    hash = ((hash << 5) - hash) + userId.charCodeAt(i);
    hash = hash & hash;
  }
  return USER_COLORS[Math.abs(hash) % USER_COLORS.length];
}

// Single remote cursor component
function RemoteCursorWidget({ 
  cursor, 
  lineHeight, 
  charWidth,
  scrollTop,
  scrollLeft 
}: { 
  cursor: RemoteCursor;
  lineHeight: number;
  charWidth: number;
  scrollTop: number;
  scrollLeft: number;
}) {
  const top = (cursor.line - 1) * lineHeight - scrollTop;
  const left = (cursor.column - 1) * charWidth - scrollLeft;
  
  // Don't render if outside viewport
  if (top < -lineHeight || top > 800) return null;
  
  return (
    <div
      className="absolute pointer-events-none z-50 transition-all duration-75"
      style={{
        top: `${top}px`,
        left: `${left}px`,
      }}
    >
      {/* Cursor line */}
      <div
        className="absolute w-0.5 h-5"
        style={{ backgroundColor: cursor.color }}
      />
      
      {/* Name label */}
      <div
        className="absolute top-0 left-0.5 px-1.5 py-0.5 text-xs font-medium text-white rounded whitespace-nowrap transform -translate-y-full"
        style={{ backgroundColor: cursor.color }}
      >
        {cursor.userName}
      </div>
      
      {/* Selection highlight (if any) */}
      {cursor.selection && (
        <div
          className="absolute opacity-20 rounded"
          style={{
            backgroundColor: cursor.color,
            top: 0,
            left: 0,
            width: `${(cursor.selection.endColumn - cursor.selection.startColumn) * charWidth}px`,
            height: `${(cursor.selection.endLine - cursor.selection.startLine + 1) * lineHeight}px`,
          }}
        />
      )}
    </div>
  );
}

// Main presence overlay component
export function EditorPresence({ 
  editorContainerRef,
  roomId,
  currentUserId,
  currentUserName,
}: EditorPresenceProps) {
  const [cursors, setCursors] = useState<RemoteCursor[]>([]);
  const [lineHeight, setLineHeight] = useState(19);
  const [charWidth, setCharWidth] = useState(8);
  const [scroll, setScroll] = useState({ top: 0, left: 0 });

  // Listen for presence updates
  useEffect(() => {
    if (!roomId) return;

    // Subscribe to cursor updates
    const unlisten = listen<{ 
      type: string;
      data: RemoteCursor[] 
    }>('collab:presence', (event) => {
      if (event.payload.type === 'cursors') {
        // Filter out current user
        const remoteCursors = event.payload.data.filter(
          c => c.userId !== currentUserId
        );
        setCursors(remoteCursors);
      }
    });

    // Fetch initial cursors
    invoke<RemoteCursor[]>('get_room_cursors', { roomId })
      .then(cursors => {
        setCursors(cursors.filter(c => c.userId !== currentUserId));
      })
      .catch(console.error);

    return () => {
      unlisten.then(fn => fn());
    };
  }, [roomId, currentUserId]);

  // Track editor scroll
  useEffect(() => {
    const container = editorContainerRef.current;
    if (!container) return;

    const handleScroll = () => {
      setScroll({
        top: container.scrollTop,
        left: container.scrollLeft,
      });
    };

    container.addEventListener('scroll', handleScroll);
    return () => container.removeEventListener('scroll', handleScroll);
  }, [editorContainerRef]);

  // Broadcast local cursor
  const broadcastCursor = useCallback(async (line: number, column: number) => {
    if (!roomId || !currentUserId) return;
    
    try {
        await invoke('broadcast_cursor', {
          roomId,
          cursor: {
            line,
            column,
            userId: currentUserId ?? currentUserName,
          }
        });
    } catch (e) {
      console.error('Failed to broadcast cursor:', e);
    }
  }, [roomId, currentUserId]);

  // Listen for Monaco cursor changes (set up by parent)
  useEffect(() => {
    const handleCursorChange = (e: CustomEvent<{ line: number; column: number }>) => {
      broadcastCursor(e.detail.line, e.detail.column);
    };
    
    window.addEventListener('kyro:cursor-change', handleCursorChange as EventListener);
    return () => {
      window.removeEventListener('kyro:cursor-change', handleCursorChange as EventListener);
    };
  }, [broadcastCursor]);

  if (!roomId || cursors.length === 0) return null;

  return (
    <div className="absolute inset-0 overflow-hidden pointer-events-none">
      {cursors.map(cursor => (
        <RemoteCursorWidget
          key={cursor.userId}
          cursor={cursor}
          lineHeight={lineHeight}
          charWidth={charWidth}
          scrollTop={scroll.top}
          scrollLeft={scroll.left}
        />
      ))}
    </div>
  );
}

// Hook for broadcasting cursor position
export function useCursorBroadcast(roomId?: string, userId?: string) {
  const broadcast = useCallback(async (line: number, column: number, selection?: {
    startLine: number;
    startColumn: number;
    endLine: number;
    endColumn: number;
  }) => {
    if (!roomId || !userId) return;

    try {
      await invoke('broadcast_cursor', {
        roomId,
        cursor: {
          userId,
          line,
          column,
          selection,
        }
      });
    } catch (e) {
      console.error('Failed to broadcast cursor:', e);
    }
  }, [roomId, userId]);

  return broadcast;
}

// User presence indicator for status bar
export function PresenceIndicator({ users }: { users: { id: string; name: string; color: string }[] }) {
  if (users.length === 0) return null;

  return (
    <div className="flex items-center gap-1">
      <div className="flex -space-x-2">
        {users.slice(0, 4).map(user => (
          <div
            key={user.id}
            className="w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold text-white border border-[#30363d]"
            style={{ backgroundColor: user.color }}
            title={user.name}
          >
            {user.name[0].toUpperCase()}
          </div>
        ))}
        {users.length > 4 && (
          <div className="w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold text-[#8b949e] bg-[#21262d] border border-[#30363d]">
            +{users.length - 4}
          </div>
        )}
      </div>
    </div>
  );
}

// Export color helper
export { getUserColor };
