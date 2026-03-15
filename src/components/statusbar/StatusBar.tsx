'use client';

import React from 'react';
import { useKyroStore } from '@/store/kyroStore';
import { GitBranch, Terminal, Sparkles, Cpu, Zap, CheckCircle, XCircle, AlertTriangle, AlertCircle } from 'lucide-react';

export function StatusBar() {
  const { 
    cursorPosition, 
    openFiles, 
    activeFileIndex, 
    gitStatus, 
    isOllamaRunning, 
    selectedModel, 
    showTerminal, 
    showChat, 
    toggleTerminal, 
    toggleChat,
    diagnosticCounts,
    // Embedded LLM state
    hardwareInfo,
    isEmbeddedLLMReady,
    isEmbeddedLLMLoading,
    selectedLocalModel,
    inferenceStats
  } = useKyroStore();
  
  const currentFile = activeFileIndex >= 0 ? openFiles[activeFileIndex] : null;

  const getModelStatus = () => {
    if (isEmbeddedLLMReady) {
      return {
        icon: <Zap size={12} className="text-green-400" />,
        text: selectedLocalModel.split('-').slice(0, 2).join('-'),
        color: 'text-green-400'
      };
    }
    if (isEmbeddedLLMLoading) {
      return {
        icon: <Cpu size={12} className="animate-pulse text-yellow-400" />,
        text: 'Loading...',
        color: 'text-yellow-400'
      };
    }
    if (isOllamaRunning) {
      return {
        icon: <Sparkles size={12} className="text-purple-400" />,
        text: selectedModel.split(':')[0],
        color: 'text-purple-400'
      };
    }
    return {
      icon: <XCircle size={12} className="text-gray-500" />,
      text: 'Offline',
      color: 'text-gray-500'
    };
  };

  const modelStatus = getModelStatus();

  return (
    <div data-testid="status-bar" className="h-6 bg-[#161b22] border-t border-[#30363d] flex items-center px-2 justify-between text-xs">
      <div className="flex items-center gap-3">
        {gitStatus && (
          <div className="flex items-center gap-1 text-[#8b949e]">
            <GitBranch size={12} />
            <span>{gitStatus.branch}</span>
          </div>
        )}

        {/* Error & Warning counts */}
        <div className="flex items-center gap-2">
          <span className={`flex items-center gap-0.5 ${diagnosticCounts.errors > 0 ? 'text-red-400' : 'text-[#8b949e]'}`}>
            <AlertCircle size={12} />
            {diagnosticCounts.errors}
          </span>
          <span className={`flex items-center gap-0.5 ${diagnosticCounts.warnings > 0 ? 'text-yellow-400' : 'text-[#8b949e]'}`}>
            <AlertTriangle size={12} />
            {diagnosticCounts.warnings}
          </span>
        </div>
        
        {/* Hardware Info */}
        {hardwareInfo && (
          <div className="flex items-center gap-1 text-[#8b949e]">
            <Cpu size={12} />
            <span className="capitalize">{hardwareInfo.backend}</span>
            {hardwareInfo.gpu_name && (
              <span className="text-[#6e7681]">| {hardwareInfo.gpu_name}</span>
            )}
          </div>
        )}
      </div>
      
      <div className="flex items-center gap-3">
        {/* Inference Stats */}
        {isEmbeddedLLMReady && inferenceStats.avgTokensPerSecond > 0 && (
          <div className="flex items-center gap-1 text-[#58a6ff]">
            <Zap size={12} />
            <span>{inferenceStats.avgTokensPerSecond.toFixed(1)} tok/s</span>
          </div>
        )}
        
        {/* Model Status */}
        <button 
          onClick={toggleChat} 
          className={`flex items-center gap-1 ${showChat ? 'bg-[#30363d] px-2 rounded' : ''} ${modelStatus.color} hover:text-[#c9d1d9]`}
        >
          {modelStatus.icon}
          <span>{modelStatus.text}</span>
          {isEmbeddedLLMReady && <CheckCircle size={10} className="text-green-400" />}
        </button>
        
        {/* Terminal */}
        <button 
          onClick={toggleTerminal} 
          className={`flex items-center gap-1 ${showTerminal ? 'bg-[#30363d] px-2 rounded text-[#58a6ff]' : 'text-[#8b949e]'} hover:text-[#c9d1d9]`}
        >
          <Terminal size={12} />
          <span>Terminal</span>
        </button>
        
        {/* Cursor Position */}
        {currentFile && (
          <span className="text-[#8b949e]">
            Ln {cursorPosition.line}, Col {cursorPosition.column}
          </span>
        )}
        
        {/* File Language */}
        {currentFile && (
          <span data-testid="language-indicator" className="text-[#8b949e]">
            {currentFile.language.toUpperCase()}
          </span>
        )}
        
        {/* Encoding */}
        <span className="text-[#8b949e]">UTF-8</span>
      </div>
    </div>
  );
}
