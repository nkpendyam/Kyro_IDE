'use client';

import React, { useState, useEffect } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Play, CheckCircle, XCircle, Clock, ChevronRight, ChevronDown, RefreshCw } from 'lucide-react';

interface TestResult {
  name: string;
  passed: boolean;
  duration_ms: number;
  output: string;
}

interface TestRunEvent {
  suite: string;
  total: number;
  passed: number;
  failed: number;
  duration_ms: number;
  results: TestResult[];
}

export function TestRunnerPanel() {
  const [suites, setSuites] = useState<TestRunEvent[]>([]);
  const [expandedSuite, setExpandedSuite] = useState<string | null>(null);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    listen<TestRunEvent>('test-run-complete', (event) => {
      setSuites(prev => {
        const existing = prev.findIndex(s => s.suite === event.payload.suite);
        if (existing >= 0) {
          const updated = [...prev];
          updated[existing] = event.payload;
          return updated;
        }
        return [...prev, event.payload];
      });
      setRunning(false);
    }).then(fn => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, []);

  const runTests = async () => {
    setRunning(true);
    try {
      await invoke('run_tests', { projectPath: '.', customCommand: null, testFilter: null });
    } catch (err) {
      console.error('Test run failed:', err);
      setRunning(false);
    }
  };

  const totalPassed = suites.reduce((sum, s) => sum + s.passed, 0);
  const totalFailed = suites.reduce((sum, s) => sum + s.failed, 0);
  const totalCount = suites.reduce((sum, s) => sum + s.total, 0);

  return (
    <div className="h-full flex flex-col bg-[#0d1117]">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#30363d] flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Play size={18} className="text-[#8b949e]" />
          <h3 className="text-[#c9d1d9] font-medium text-sm">Test Runner</h3>
        </div>
        <button
          onClick={runTests}
          disabled={running}
          className="flex items-center gap-1.5 px-3 py-1 text-xs bg-[#238636] hover:bg-[#2ea043] text-white rounded disabled:opacity-50"
        >
          {running ? <RefreshCw size={12} className="animate-spin" /> : <Play size={12} />}
          {running ? 'Running...' : 'Run All'}
        </button>
      </div>

      {/* Summary */}
      {totalCount > 0 && (
        <div className="px-4 py-2 border-b border-[#30363d] flex items-center gap-4 text-xs">
          <span className="text-green-400 flex items-center gap-1">
            <CheckCircle size={12} /> {totalPassed} passed
          </span>
          <span className={`flex items-center gap-1 ${totalFailed > 0 ? 'text-red-400' : 'text-[#8b949e]'}`}>
            <XCircle size={12} /> {totalFailed} failed
          </span>
          <span className="text-[#8b949e] flex items-center gap-1">
            <Clock size={12} /> {suites.reduce((sum, s) => sum + s.duration_ms, 0)}ms
          </span>
        </div>
      )}

      {/* Suite tree */}
      <div className="flex-1 overflow-y-auto p-2">
        {suites.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-[#8b949e] text-xs">
            <Play size={24} className="mb-2 opacity-40" />
            <p>No test results yet</p>
            <p className="text-[10px] mt-1">Click &quot;Run All&quot; to execute project tests</p>
          </div>
        ) : (
          suites.map(suite => {
            const isExpanded = expandedSuite === suite.suite;
            return (
              <div key={suite.suite} className="mb-1">
                <button
                  onClick={() => setExpandedSuite(isExpanded ? null : suite.suite)}
                  className="w-full flex items-center gap-2 px-2 py-1.5 rounded text-xs hover:bg-[#161b22]"
                >
                  {isExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                  {suite.failed > 0 ? (
                    <XCircle size={12} className="text-red-400" />
                  ) : (
                    <CheckCircle size={12} className="text-green-400" />
                  )}
                  <span className="text-[#c9d1d9] flex-1 text-left">{suite.suite}</span>
                  <span className="text-[#8b949e]">{suite.passed}/{suite.total}</span>
                </button>
                {isExpanded && (
                  <div className="ml-6 border-l border-[#30363d] pl-3">
                    {suite.results.map((test, i) => (
                      <div key={i} className="flex items-center gap-2 px-2 py-1 text-xs">
                        {test.passed ? (
                          <CheckCircle size={10} className="text-green-400 shrink-0" />
                        ) : (
                          <XCircle size={10} className="text-red-400 shrink-0" />
                        )}
                        <span className="text-[#c9d1d9] flex-1">{test.name}</span>
                        <span className="text-[#484f58]">{test.duration_ms}ms</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
