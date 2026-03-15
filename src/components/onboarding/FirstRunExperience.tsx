'use client';

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { 
  Sparkles, Download, Check, ChevronRight, ChevronLeft, 
  Cpu, HardDrive, Monitor, Zap, Bot, Code, Users, Puzzle,
  ArrowRight, Loader2
} from 'lucide-react';

// Steps in the first-run experience
const STEPS = [
  { id: 'welcome', title: 'Welcome', icon: Sparkles },
  { id: 'hardware', title: 'Hardware', icon: Cpu },
  { id: 'model', title: 'AI Model', icon: Bot },
  { id: 'languages', title: 'Languages', icon: Code },
  { id: 'ready', title: 'Ready', icon: Zap },
];

// Hardware info from backend
interface HardwareInfo {
  gpu_name: string | null;
  vram_gb: number;
  ram_gb: number;
  cpu_cores: number;
  backend: string;
  memory_tier: string;
  recommended_model: string;
}

// Available model
interface ModelInfo {
  id: string;
  name: string;
  size_mb: number;
  downloaded: boolean;
  loaded: boolean;
  quantization: string;
  min_memory_tier: string;
  description: string;
}

// Props
interface FirstRunExperienceProps {
  onComplete: () => void;
}

export function FirstRunExperience({ onComplete }: FirstRunExperienceProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [hardwareInfo, setHardwareInfo] = useState<HardwareInfo | null>(null);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [isDownloading, setIsDownloading] = useState(false);
  const [isComplete, setIsComplete] = useState(false);

  // Fetch hardware info
  useEffect(() => {
    invoke<HardwareInfo>('get_hardware_info')
      .then(setHardwareInfo)
      .catch(console.error);
  }, []);

  // Fetch available models
  useEffect(() => {
    invoke<{ id: string; name: string; size_mb: number; description: string; quantization: string; min_ram_gb: number }[]>('list_available_models')
      .then((catalog) => {
        setModels(catalog.map((m) => ({
          id: m.id,
          name: m.name,
          size_mb: m.size_mb,
          description: m.description,
          downloaded: m.description.startsWith('[Downloaded]'),
          loaded: false,
          quantization: m.quantization,
          min_memory_tier: m.min_ram_gb <= 4 ? 'low' : m.min_ram_gb <= 8 ? 'medium' : 'high',
        })));
      })
      .catch(console.error);
  }, []);

  // Auto-select recommended model
  useEffect(() => {
    if (hardwareInfo && models.length > 0 && !selectedModel) {
      const recommended = models.find(m => m.id === hardwareInfo.recommended_model);
      setSelectedModel(recommended?.id || models[0]?.id);
    }
  }, [hardwareInfo, models, selectedModel]);

  // Start model download with real progress events
  const startDownload = async () => {
    if (!selectedModel) return;

    setIsDownloading(true);
    setDownloadProgress(0);

    // Listen for real progress events from backend
    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<{ model_id: string; percent: number; state: string }>('model-download-progress', (event) => {
        setDownloadProgress(event.payload.percent);
        if (event.payload.state === 'Complete') {
          setDownloadProgress(100);
          setTimeout(() => {
            setCurrentStep(4);
            setIsComplete(true);
          }, 500);
        }
      });

      await invoke('download_model', { modelId: selectedModel });

      // If no events fired (e.g., model already cached), complete immediately
      setDownloadProgress(100);
      setTimeout(() => {
        setCurrentStep(4);
        setIsComplete(true);
      }, 500);
    } catch (e) {
      console.error('Failed to download model:', e);
      // Still allow completion even if download fails
      setDownloadProgress(100);
      setCurrentStep(4);
      setIsComplete(true);
    } finally {
      setIsDownloading(false);
      unlisten?.();
    }
  };

  // Complete setup
  const completeSetup = async () => {
    try {
      await invoke('save_first_run_complete');
    } catch {
      // Persist locally as fallback
      localStorage.setItem('kyro-first-run-done', 'true');
    }
    onComplete();
  };

  // Render step content
  const renderStepContent = () => {
    switch (STEPS[currentStep].id) {
      case 'welcome':
        return (
          <div className="text-center space-y-6">
            <div className="text-6xl mb-4">⚡</div>
            <h1 className="text-3xl font-bold text-[#c9d1d9]">
              Welcome to <span className="text-[#a371f7]">KRO IDE</span>
            </h1>
            <p className="text-[#8b949e] max-w-md mx-auto">
              An AI-native, GPU-accelerated code editor that works completely offline.
              Let&apos;s set things up for you.
            </p>

            <div className="grid grid-cols-2 gap-4 max-w-lg mx-auto mt-8">
              <div className="p-4 bg-[#161b22] rounded-lg border border-[#30363d] text-left">
                <Bot className="text-[#a371f7] mb-2" size={24} />
                <h3 className="font-medium text-[#c9d1d9]">AI-Powered</h3>
                <p className="text-sm text-[#8b949e]">Chat that knows your codebase</p>
              </div>
              <div className="p-4 bg-[#161b22] rounded-lg border border-[#30363d] text-left">
                <Zap className="text-[#3fb950] mb-2" size={24} />
                <h3 className="font-medium text-[#c9d1d9]">Offline First</h3>
                <p className="text-sm text-[#8b949e]">Works without internet</p>
              </div>
              <div className="p-4 bg-[#161b22] rounded-lg border border-[#30363d] text-left">
                <Users className="text-[#58a6ff] mb-2" size={24} />
                <h3 className="font-medium text-[#c9d1d9]">Collaborative</h3>
                <p className="text-sm text-[#8b949e]">Real-time editing with E2E</p>
              </div>
              <div className="p-4 bg-[#161b22] rounded-lg border border-[#30363d] text-left">
                <Puzzle className="text-[#d29922] mb-2" size={24} />
                <h3 className="font-medium text-[#c9d1d9]">Extensible</h3>
                <p className="text-sm text-[#8b949e]">VS Code extension support</p>
              </div>
            </div>
          </div>
        );

      case 'hardware':
        return (
          <div className="space-y-6">
            <div className="text-center">
              <h2 className="text-2xl font-bold text-[#c9d1d9]">Hardware Detected</h2>
              <p className="text-[#8b949e]">We found the following capabilities</p>
            </div>

            {hardwareInfo ? (
              <div className="grid grid-cols-2 gap-4 max-w-lg mx-auto">
                <div className="p-4 bg-[#161b22] rounded-lg border border-[#30363d]">
                  <div className="flex items-center gap-2 mb-2">
                    <Monitor className="text-[#58a6ff]" size={20} />
                    <span className="text-sm text-[#8b949e]">GPU</span>
                  </div>
                  <p className="font-medium text-[#c9d1d9]">
                    {hardwareInfo.gpu_name || 'CPU Only'}
                  </p>
                </div>

                <div className="p-4 bg-[#161b22] rounded-lg border border-[#30363d]">
                  <div className="flex items-center gap-2 mb-2">
                    <HardDrive className="text-[#3fb950]" size={20} />
                    <span className="text-sm text-[#8b949e]">VRAM</span>
                  </div>
                  <p className="font-medium text-[#c9d1d9]">
                    {hardwareInfo.vram_gb} GB
                  </p>
                </div>

                <div className="p-4 bg-[#161b22] rounded-lg border border-[#30363d]">
                  <div className="flex items-center gap-2 mb-2">
                    <Cpu className="text-[#d29922]" size={20} />
                    <span className="text-sm text-[#8b949e]">RAM</span>
                  </div>
                  <p className="font-medium text-[#c9d1d9]">
                    {hardwareInfo.ram_gb} GB
                  </p>
                </div>

                <div className="p-4 bg-[#161b22] rounded-lg border border-[#30363d]">
                  <div className="flex items-center gap-2 mb-2">
                    <Zap className="text-[#a371f7]" size={20} />
                    <span className="text-sm text-[#8b949e]">Backend</span>
                  </div>
                  <p className="font-medium text-[#c9d1d9] capitalize">
                    {hardwareInfo.backend}
                  </p>
                </div>
              </div>
            ) : (
              <div className="flex justify-center">
                <Loader2 className="animate-spin text-[#58a6ff]" size={32} />
              </div>
            )}

            <div className="text-center text-sm text-[#8b949e]">
              Memory tier: <span className="text-[#3fb950] font-medium">{hardwareInfo?.memory_tier || 'Detecting...'}</span>
            </div>
          </div>
        );

      case 'model':
        return (
          <div className="space-y-6">
            <div className="text-center">
              <h2 className="text-2xl font-bold text-[#c9d1d9]">Choose AI Model</h2>
              <p className="text-[#8b949e]">Download a model for offline AI assistance</p>
            </div>

            <div className="space-y-3 max-w-md mx-auto">
              {models.map(model => (
                <button
                  key={model.id}
                  onClick={() => setSelectedModel(model.id)}
                  className={`w-full p-4 rounded-lg border text-left transition-all ${
                    selectedModel === model.id
                      ? 'border-[#a371f7] bg-[#a371f720]'
                      : 'border-[#30363d] bg-[#161b22] hover:border-[#484f58]'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="font-medium text-[#c9d1d9]">{model.name}</p>
                      <p className="text-sm text-[#8b949e]">{model.description}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-sm text-[#8b949e]">{model.size_mb} MB</p>
                      {model.downloaded && (
                        <span className="text-xs text-[#3fb950]">Downloaded</span>
                      )}
                    </div>
                  </div>
                  {selectedModel === model.id && (
                    <div className="mt-2 flex items-center gap-1 text-[#a371f7]">
                      <Check size={14} />
                      <span className="text-sm">Selected</span>
                    </div>
                  )}
                </button>
              ))}
            </div>

            {isDownloading && (
              <div className="max-w-md mx-auto">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-[#8b949e]">Downloading model...</span>
                  <span className="text-sm text-[#c9d1d9]">{downloadProgress}%</span>
                </div>
                <div className="h-2 bg-[#21262d] rounded-full overflow-hidden">
                  <div
                    className="h-full bg-[#a371f7] transition-all duration-300"
                    style={{ width: `${downloadProgress}%` }}
                  />
                </div>
              </div>
            )}

            <div className="flex justify-center">
              <button
                onClick={startDownload}
                disabled={!selectedModel || isDownloading}
                className="px-6 py-2 bg-[#238636] hover:bg-[#2ea043] disabled:bg-[#21262d] disabled:text-[#8b949e] text-white rounded-lg flex items-center gap-2"
              >
                {isDownloading ? (
                  <>
                    <Loader2 className="animate-spin" size={16} />
                    Downloading...
                  </>
                ) : (
                  <>
                    <Download size={16} />
                    Download & Continue
                  </>
                )}
              </button>
            </div>
          </div>
        );

      case 'languages':
        return (
          <div className="space-y-6">
            <div className="text-center">
              <h2 className="text-2xl font-bold text-[#c9d1d9]">Language Support</h2>
              <p className="text-[#8b949e]">KRO IDE supports 165+ languages out of the box</p>
            </div>

            <div className="grid grid-cols-4 gap-3 max-w-lg mx-auto">
              {['Rust', 'TypeScript', 'Python', 'Go', 'Java', 'C++', 'Ruby', 'PHP', 'Swift', 'Kotlin', 'Scala', 'Lua'].map(lang => (
                <div
                  key={lang}
                  className="p-2 bg-[#161b22] rounded border border-[#30363d] text-center"
                >
                  <span className="text-sm text-[#c9d1d9]">{lang}</span>
                </div>
              ))}
            </div>

            <div className="text-center text-sm text-[#8b949e]">
              + 150 more languages with Tree-sitter syntax highlighting
            </div>

            <div className="flex justify-center">
              <button
                onClick={() => setCurrentStep(4)}
                className="px-6 py-2 bg-[#238636] hover:bg-[#2ea043] text-white rounded-lg flex items-center gap-2"
              >
                Continue
                <ArrowRight size={16} />
              </button>
            </div>
          </div>
        );

      case 'ready':
        return (
          <div className="text-center space-y-6">
            <div className="text-6xl">🎉</div>
            <h2 className="text-2xl font-bold text-[#c9d1d9]">You&apos;re all set!</h2>
            <p className="text-[#8b949e] max-w-md mx-auto">
              KRO IDE is ready to use. Open a project folder to get started with AI-powered coding.
            </p>

            <div className="bg-[#161b22] rounded-lg border border-[#30363d] p-4 max-w-md mx-auto text-left">
              <h3 className="font-medium text-[#c9d1d9] mb-3">Quick Tips:</h3>
              <ul className="space-y-2 text-sm text-[#8b949e]">
                <li className="flex items-center gap-2">
                  <kbd className="px-2 py-0.5 bg-[#21262d] rounded text-xs">Ctrl+K</kbd>
                  <span>Open inline chat</span>
                </li>
                <li className="flex items-center gap-2">
                  <kbd className="px-2 py-0.5 bg-[#21262d] rounded text-xs">Ctrl+P</kbd>
                  <span>Quick file search</span>
                </li>
                <li className="flex items-center gap-2">
                  <kbd className="px-2 py-0.5 bg-[#21262d] rounded text-xs">Ctrl+Shift+P</kbd>
                  <span>Command palette</span>
                </li>
                <li className="flex items-center gap-2">
                  <kbd className="px-2 py-0.5 bg-[#21262d] rounded text-xs">F5</kbd>
                  <span>Start debugging</span>
                </li>
              </ul>
            </div>

            <button
              onClick={completeSetup}
              className="px-8 py-3 bg-[#a371f7] hover:bg-[#8957b3] text-white rounded-lg font-medium flex items-center gap-2 mx-auto"
            >
              Start Coding
              <Sparkles size={18} />
            </button>
          </div>
        );

      default:
        return null;
    }
  };

  return (
    <div className="fixed inset-0 bg-[#0d1117] flex flex-col z-50">
      {/* Progress bar */}
      <div className="h-1 bg-[#21262d]">
        <div
          className="h-full bg-[#a371f7] transition-all duration-300"
          style={{ width: `${((currentStep + 1) / STEPS.length) * 100}%` }}
        />
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col items-center justify-center p-8">
        {renderStepContent()}
      </div>

      {/* Navigation */}
      {!isComplete && currentStep > 0 && currentStep < 3 && (
        <div className="absolute bottom-8 left-1/2 -translate-x-1/2 flex items-center gap-4">
          {currentStep > 0 && (
            <button
              onClick={() => setCurrentStep(prev => prev - 1)}
              className="px-4 py-2 text-[#8b949e] hover:text-[#c9d1d9] flex items-center gap-1"
            >
              <ChevronLeft size={16} />
              Back
            </button>
          )}
          
          {currentStep < STEPS.length - 1 && STEPS[currentStep].id !== 'model' && (
            <button
              onClick={() => setCurrentStep(prev => prev + 1)}
              className="px-4 py-2 bg-[#238636] hover:bg-[#2ea043] text-white rounded-lg flex items-center gap-1"
            >
              Next
              <ChevronRight size={16} />
            </button>
          )}
        </div>
      )}

      {/* Step indicators */}
      <div className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-2">
        {STEPS.map((step, idx) => (
          <div
            key={step.id}
            className={`w-2 h-2 rounded-full transition-all ${
              idx === currentStep
                ? 'bg-[#a371f7] w-4'
                : idx < currentStep
                ? 'bg-[#3fb950]'
                : 'bg-[#30363d]'
            }`}
          />
        ))}
      </div>
    </div>
  );
}

export default FirstRunExperience;
