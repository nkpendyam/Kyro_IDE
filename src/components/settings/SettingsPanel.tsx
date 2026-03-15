'use client';

import React, { useCallback, useEffect, useState } from 'react';
import { useExtendedKyroStore } from '@/store/extendedStore';
import { useKyroStore } from '@/store/kyroStore';
import { invoke } from '@tauri-apps/api/core';
import { Settings, Sun, Moon, Monitor, Save, RotateCcw, Sparkles, Search } from 'lucide-react';

// Reusable toggle switch component
function Toggle({ value, onChange, label }: { value: boolean; onChange: (v: boolean) => void; label?: string }) {
  return (
    <button
      onClick={() => onChange(!value)}
      className={`w-12 h-6 rounded-full relative ${value ? 'bg-[#238636]' : 'bg-[#21262d]'}`}
      aria-label={label}
    >
      <div className={`absolute top-1 w-4 h-4 rounded-full bg-white transition-transform ${value ? 'translate-x-7' : 'translate-x-1'}`} />
    </button>
  );
}

export function SettingsPanel() {
  const { theme, settings, updateSettings, setTheme, updateChannel, autoUpdateEnabled, setUpdateChannel } = useExtendedKyroStore();
  const editorOptions = useKyroStore(s => s.settings.editorOptions);
  const setEditorOptions = useKyroStore(s => s.setEditorOptions);
  const setKyroTheme = useKyroStore(s => s.setTheme);
  const ghostTextConfig = useKyroStore(s => s.ghostTextConfig);
  const setGhostTextConfig = useKyroStore(s => s.setGhostTextConfig);
  const [saving, setSaving] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // Load settings from backend on mount
  useEffect(() => {
    invoke<Record<string, unknown>>('get_settings')
      .then((saved) => {
        updateSettings({
          fontSize: typeof saved.fontSize === 'number' ? saved.fontSize : 14,
          tabSize: typeof saved.tabSize === 'number' ? saved.tabSize : 4,
          wordWrap: typeof saved.wordWrap === 'boolean' ? saved.wordWrap : true,
          minimap: typeof saved.minimap === 'boolean' ? saved.minimap : true,
          formatOnSave: typeof saved.formatOnSave === 'boolean' ? saved.formatOnSave : false,
          autoSave: typeof saved.autoSave === 'boolean' ? saved.autoSave : true,
          autoSaveDelay: typeof saved.autoSaveDelay === 'number' ? saved.autoSaveDelay : 1000,
        });
        const savedTheme = saved.theme as string | undefined;
        if (savedTheme === 'light' || savedTheme === 'dark' || savedTheme === 'system') {
          setTheme(savedTheme);
        }

        setEditorOptions({
          fontSize: typeof saved.fontSize === 'number' ? saved.fontSize : 14,
          tabSize: typeof saved.tabSize === 'number' ? saved.tabSize : 4,
          wordWrap: saved.wordWrap === 'off' || saved.wordWrap === 'bounded' ? saved.wordWrap : 'on',
          minimap: typeof saved.minimap === 'boolean' ? saved.minimap : true,
          formatOnSave: typeof saved.formatOnSave === 'boolean' ? saved.formatOnSave : true,
          autoSave: saved.autoSave === 'off' || saved.autoSave === 'onFocusChange' ? saved.autoSave : 'afterDelay',
          lineNumbers: saved.lineNumbers === 'off' || saved.lineNumbers === 'relative' ? saved.lineNumbers : 'on',
          renderWhitespace: saved.renderWhitespace === 'none' || saved.renderWhitespace === 'boundary' || saved.renderWhitespace === 'all'
            ? saved.renderWhitespace
            : 'selection',
          bracketPairColorization: typeof saved.bracketPairColorization === 'boolean' ? saved.bracketPairColorization : true,
          stickyScroll: typeof saved.stickyScroll === 'boolean' ? saved.stickyScroll : true,
          inlineSuggest: typeof saved.inlineSuggest === 'boolean' ? saved.inlineSuggest : true,
        });

        setGhostTextConfig({
          enabled: typeof saved.ghostTextEnabled === 'boolean' ? saved.ghostTextEnabled : true,
          temperature: typeof saved.ghostTextTemperature === 'number' ? saved.ghostTextTemperature : ghostTextConfig.temperature,
          maxTokens: typeof saved.ghostTextMaxTokens === 'number' ? saved.ghostTextMaxTokens : ghostTextConfig.maxTokens,
          debounceMs: typeof saved.ghostTextDebounceMs === 'number' ? saved.ghostTextDebounceMs : ghostTextConfig.debounceMs,
          cacheEnabled: typeof saved.ghostTextCacheEnabled === 'boolean' ? saved.ghostTextCacheEnabled : ghostTextConfig.cacheEnabled,
        });
      })
      .catch(() => {
        // Backend unavailable (web mode) — keep store defaults
      });
  }, [updateSettings, setTheme]);

  const matchesSearch = useCallback((terms: string[]) => {
    if (!searchQuery.trim()) return true;
    const query = searchQuery.trim().toLowerCase();
    return terms.some(term => term.toLowerCase().includes(query));
  }, [searchQuery]);

  const showAppearance = matchesSearch(['appearance', 'theme', 'light', 'dark', 'system']);
  const showEditor = matchesSearch(['editor', 'font size', 'tab size', 'word wrap', 'minimap', 'format on save', 'auto save']);
  const showAdvancedEditor = matchesSearch([
    'advanced editor',
    'sticky scroll',
    'bracket colorization',
    'inline suggestions',
    'render whitespace',
    'line numbers',
    'auto save mode',
    'format on save',
  ]);
  const showAi = matchesSearch(['ai', 'ghost text', 'temperature', 'max tokens', 'debounce', 'cache']);
  const showUpdates = matchesSearch(['updates', 'update channel', 'auto update', 'stable', 'beta', 'nightly']);

  const handleSave = async () => {
    setSaving(true);
    try {
      // Persist entire settings object atomically via save_settings
      const payload: Record<string, unknown> = {
        ...settings,
        theme,
        lineNumbers: editorOptions.lineNumbers,
        renderWhitespace: editorOptions.renderWhitespace,
        bracketPairColorization: editorOptions.bracketPairColorization,
        stickyScroll: editorOptions.stickyScroll,
        inlineSuggest: editorOptions.inlineSuggest,
        ghostTextEnabled: ghostTextConfig.enabled,
        ghostTextTemperature: ghostTextConfig.temperature,
        ghostTextMaxTokens: ghostTextConfig.maxTokens,
        ghostTextDebounceMs: ghostTextConfig.debounceMs,
        ghostTextCacheEnabled: ghostTextConfig.cacheEnabled,
      };
      await invoke('save_settings', { settings: payload });
      setKyroTheme(theme);
    } catch {
      // Fallback: save to localStorage if backend unavailable
      localStorage.setItem('kyro-settings', JSON.stringify({ ...settings, theme }));
    }
    setSaving(false);
  };

  const handleReset = () => {
    updateSettings({
      fontSize: 14,
      tabSize: 4,
      wordWrap: true,
      minimap: true,
      formatOnSave: true,
      autoSave: true,
      autoSaveDelay: 1000,
    });
  };

  return (
    <div className="h-full flex flex-col bg-[#0d1117]">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#30363d] flex items-center gap-2">
        <Settings size={18} className="text-[#8b949e]" />
        <h3 className="text-[#c9d1d9] font-medium">Settings</h3>
      </div>

      <div className="px-4 py-3 border-b border-[#30363d]">
        <div className="relative">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#8b949e]" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search settings..."
            className="w-full pl-9 pr-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm text-[#c9d1d9] placeholder-[#8b949e] focus:outline-none focus:border-[#58a6ff]"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-6">
        {/* Appearance */}
        {showAppearance && (
        <section>
          <h4 className="text-sm text-[#c9d1d9] font-medium mb-3">Appearance</h4>
          
          {/* Theme */}
          <div className="space-y-2">
            <label className="text-sm text-[#8b949e]">Theme</label>
            <div className="flex gap-2">
              <button
                onClick={() => setTheme('light')}
                className={`flex items-center gap-2 px-3 py-2 rounded border ${
                  theme === 'light'
                    ? 'bg-[#21262d] border-[#58a6ff] text-[#c9d1d9]'
                    : 'border-[#30363d] text-[#8b949e] hover:border-[#58a6ff]'
                }`}
              >
                <Sun size={16} />
                Light
              </button>
              <button
                onClick={() => setTheme('dark')}
                className={`flex items-center gap-2 px-3 py-2 rounded border ${
                  theme === 'dark'
                    ? 'bg-[#21262d] border-[#58a6ff] text-[#c9d1d9]'
                    : 'border-[#30363d] text-[#8b949e] hover:border-[#58a6ff]'
                }`}
              >
                <Moon size={16} />
                Dark
              </button>
              <button
                onClick={() => setTheme('system')}
                className={`flex items-center gap-2 px-3 py-2 rounded border ${
                  theme === 'system'
                    ? 'bg-[#21262d] border-[#58a6ff] text-[#c9d1d9]'
                    : 'border-[#30363d] text-[#8b949e] hover:border-[#58a6ff]'
                }`}
              >
                <Monitor size={16} />
                System
              </button>
            </div>
          </div>
        </section>
        )}

        {/* Editor */}
        {showEditor && (
        <section>
          <h4 className="text-sm text-[#c9d1d9] font-medium mb-3">Editor</h4>
          
          <div className="space-y-4">
            {/* Font Size */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Font Size</label>
              <input
                type="number"
                value={settings.fontSize}
                onChange={(e) => updateSettings({ fontSize: parseInt(e.target.value) || 14 })}
                min={8}
                max={32}
                className="w-20 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              />
            </div>

            {/* Tab Size */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Tab Size</label>
              <select
                value={settings.tabSize}
                onChange={(e) => updateSettings({ tabSize: parseInt(e.target.value) })}
                className="w-20 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              >
                <option value={2}>2</option>
                <option value={4}>4</option>
                <option value={8}>8</option>
              </select>
            </div>

            {/* Word Wrap */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Word Wrap</label>
              <Toggle value={settings.wordWrap} onChange={(v) => updateSettings({ wordWrap: v })} label="Word Wrap" />
            </div>

            {/* Minimap */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Show Minimap</label>
              <Toggle value={settings.minimap} onChange={(v) => updateSettings({ minimap: v })} label="Show Minimap" />
            </div>

            {/* Format on Save */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Format on Save</label>
              <Toggle value={settings.formatOnSave} onChange={(v) => updateSettings({ formatOnSave: v })} label="Format on Save" />
            </div>

            {/* Auto Save */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Auto Save</label>
              <Toggle value={settings.autoSave} onChange={(v) => updateSettings({ autoSave: v })} label="Auto Save" />
            </div>
          </div>
        </section>
        )}

        {/* Advanced Editor */}
        {showAdvancedEditor && (
        <section>
          <h4 className="text-sm text-[#c9d1d9] font-medium mb-3">Advanced Editor</h4>
          
          <div className="space-y-4">
            {/* Sticky Scroll */}
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm text-[#8b949e]">Sticky Scroll</label>
                <p className="text-[10px] text-[#484f58]">Pin scope headers at top of editor</p>
              </div>
              <Toggle value={editorOptions.stickyScroll} onChange={(v) => setEditorOptions({ stickyScroll: v })} label="Sticky Scroll" />
            </div>

            {/* Bracket Pair Colorization */}
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm text-[#8b949e]">Bracket Colorization</label>
                <p className="text-[10px] text-[#484f58]">Colorize matching bracket pairs</p>
              </div>
              <Toggle value={editorOptions.bracketPairColorization} onChange={(v) => setEditorOptions({ bracketPairColorization: v })} label="Bracket Colorization" />
            </div>

            {/* Inline Suggest */}
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm text-[#8b949e]">Inline Suggestions</label>
                <p className="text-[10px] text-[#484f58]">Show AI completions inline</p>
              </div>
              <Toggle value={editorOptions.inlineSuggest} onChange={(v) => setEditorOptions({ inlineSuggest: v })} label="Inline Suggest" />
            </div>

            {/* Render Whitespace */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Render Whitespace</label>
              <select
                value={editorOptions.renderWhitespace}
                onChange={(e) => setEditorOptions({ renderWhitespace: e.target.value as 'none' | 'boundary' | 'selection' | 'all' })}
                className="w-28 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              >
                <option value="none">None</option>
                <option value="boundary">Boundary</option>
                <option value="selection">Selection</option>
                <option value="all">All</option>
              </select>
            </div>

            {/* Line Numbers */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Line Numbers</label>
              <select
                value={editorOptions.lineNumbers}
                onChange={(e) => setEditorOptions({ lineNumbers: e.target.value as 'on' | 'off' | 'relative' })}
                className="w-28 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              >
                <option value="on">On</option>
                <option value="off">Off</option>
                <option value="relative">Relative</option>
              </select>
            </div>

            {/* Auto Save mode */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Auto Save Mode</label>
              <select
                value={editorOptions.autoSave}
                onChange={(e) => setEditorOptions({ autoSave: e.target.value as 'off' | 'afterDelay' | 'onFocusChange' })}
                className="w-32 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              >
                <option value="off">Off</option>
                <option value="afterDelay">After Delay</option>
                <option value="onFocusChange">On Focus Change</option>
              </select>
            </div>

            {/* Format on Save */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Format on Save</label>
              <Toggle value={editorOptions.formatOnSave} onChange={(v) => setEditorOptions({ formatOnSave: v })} label="Format on Save" />
            </div>
          </div>
        </section>
        )}

        {/* AI Settings */}
        {showAi && (
        <section>
          <h4 className="text-sm text-[#c9d1d9] font-medium mb-3 flex items-center gap-1.5">
            <Sparkles size={14} className="text-[#a371f7]" /> AI / Ghost Text
          </h4>
          
          <div className="space-y-4">
            {/* Ghost Text Enabled */}
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm text-[#8b949e]">Ghost Text</label>
                <p className="text-[10px] text-[#484f58]">AI-powered inline completions</p>
              </div>
              <Toggle value={ghostTextConfig.enabled} onChange={(v) => setGhostTextConfig({ enabled: v })} label="Ghost Text" />
            </div>

            {/* Temperature */}
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm text-[#8b949e]">Temperature</label>
                <p className="text-[10px] text-[#484f58]">Lower = more focused, higher = more creative</p>
              </div>
              <input
                type="number"
                value={ghostTextConfig.temperature}
                onChange={(e) => setGhostTextConfig({ temperature: Math.max(0, Math.min(2, parseFloat(e.target.value) || 0)) })}
                min={0}
                max={2}
                step={0.1}
                className="w-20 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              />
            </div>

            {/* Max Tokens */}
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm text-[#8b949e]">Max Tokens</label>
                <p className="text-[10px] text-[#484f58]">Completion length limit</p>
              </div>
              <input
                type="number"
                value={ghostTextConfig.maxTokens}
                onChange={(e) => setGhostTextConfig({ maxTokens: Math.max(10, parseInt(e.target.value) || 100) })}
                min={10}
                max={500}
                step={10}
                className="w-20 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              />
            </div>

            {/* Debounce */}
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm text-[#8b949e]">Debounce (ms)</label>
                <p className="text-[10px] text-[#484f58]">Delay before triggering completion</p>
              </div>
              <input
                type="number"
                value={ghostTextConfig.debounceMs}
                onChange={(e) => setGhostTextConfig({ debounceMs: Math.max(50, parseInt(e.target.value) || 200) })}
                min={50}
                max={2000}
                step={50}
                className="w-20 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              />
            </div>

            {/* Cache */}
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm text-[#8b949e]">Completion Cache</label>
                <p className="text-[10px] text-[#484f58]">Cache completions for faster repeats</p>
              </div>
              <Toggle value={ghostTextConfig.cacheEnabled} onChange={(v) => setGhostTextConfig({ cacheEnabled: v })} label="Cache" />
            </div>
          </div>
        </section>
        )}

        {/* Updates */}
        {showUpdates && (
        <section>
          <h4 className="text-sm text-[#c9d1d9] font-medium mb-3">Updates</h4>
          
          <div className="space-y-4">
            {/* Update Channel */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Update Channel</label>
              <select
                value={updateChannel}
                onChange={(e) => setUpdateChannel(e.target.value)}
                className="w-28 bg-[#161b22] border border-[#30363d] rounded px-3 py-1 text-[#c9d1d9] text-sm"
              >
                <option value="stable">Stable</option>
                <option value="beta">Beta</option>
                <option value="nightly">Nightly</option>
              </select>
            </div>

            {/* Auto Update */}
            <div className="flex items-center justify-between">
              <label className="text-sm text-[#8b949e]">Auto Update</label>
              <button
                onClick={async () => {
                  await invoke('set_auto_update', { enabled: !autoUpdateEnabled });
                }}
                className={`w-12 h-6 rounded-full relative ${
                  autoUpdateEnabled ? 'bg-[#238636]' : 'bg-[#21262d]'
                }`}
              >
                <div
                  className={`absolute top-1 w-4 h-4 rounded-full bg-white transition-transform ${
                    autoUpdateEnabled ? 'translate-x-7' : 'translate-x-1'
                  }`}
                />
              </button>
            </div>
          </div>
        </section>
        )}

        {!showAppearance && !showEditor && !showAdvancedEditor && !showAi && !showUpdates && (
          <div className="rounded border border-[#30363d] bg-[#161b22] p-4 text-sm text-[#8b949e]">
            No settings matched "{searchQuery}".
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-3 border-t border-[#30363d] flex justify-end gap-2">
        <button
          onClick={handleReset}
          className="flex items-center gap-2 px-4 py-2 text-sm text-[#8b949e] hover:text-[#c9d1d9] hover:bg-[#21262d] rounded"
        >
          <RotateCcw size={16} />
          Reset
        </button>
        <button
          onClick={handleSave}
          disabled={saving}
          className="flex items-center gap-2 px-4 py-2 text-sm bg-[#238636] hover:bg-[#2ea043] text-white rounded"
        >
          <Save size={16} />
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  );
}
