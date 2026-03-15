'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Input } from '@/components/ui/input';
import { 
  Bot, 
  Rocket, 
  FileCode, 
  Brain, 
  CheckCircle2, 
  Clock, 
  AlertCircle,
  Play,
  Pause,
  RotateCcw,
  Zap,
  Target,
  TrendingUp,
  Activity,
  Sparkles,
  ChevronRight,
  Terminal,
  Code2,
  Database,
  GitBranch,
  Layers,
  Cpu,
  MemoryStick
} from 'lucide-react';

// Mock data
interface Agent {
  id: string;
  name: string;
  type: 'coder' | 'analyst' | 'tester' | 'reviewer';
  status: 'idle' | 'working' | 'completed' | 'error';
  currentTask?: string;
  progress?: number;
  completedTasks: number;
  successRate: number;
}

// Orchestrator mission (from Tauri)
interface OrchestratorMission {
  id: string;
  goal: string;
  constraints: string[];
  phase: 'plan' | 'edit' | 'test' | 'review' | 'deploy';
  status: 'running' | 'paused' | 'completed' | 'failed';
  assigned_agents: string[];
  artifacts: { kind: string; path?: string; content?: string }[];
  created_at: string;
  updated_at: string;
}

interface Mission {
  id: string;
  title: string;
  description: string;
  status: 'pending' | 'in_progress' | 'completed' | 'failed';
  priority: 'low' | 'medium' | 'high' | 'critical';
  assignedAgents: string[];
  progress: number;
  createdAt: Date;
  artifacts: string[];
  phase?: string;
}

interface Artifact {
  id: string;
  name: string;
  type: 'code' | 'test' | 'documentation' | 'config';
  path: string;
  createdAt: Date;
  verified: boolean;
  size: string;
}

interface KnowledgeEntry {
  id: string;
  title: string;
  category: 'pattern' | 'solution' | 'error' | 'optimization';
  relevance: number;
  lastUsed: Date;
  usageCount: number;
}

function toMission(m: OrchestratorMission): Mission {
  const phaseProgress = { plan: 10, edit: 40, test: 60, review: 80, deploy: 100 }[m.phase] ?? 0;
  return {
    id: m.id,
    title: m.goal.slice(0, 60) + (m.goal.length > 60 ? '...' : ''),
    description: m.goal,
    status: m.status === 'running' ? 'in_progress' : m.status === 'completed' ? 'completed' : m.status === 'failed' ? 'failed' : 'pending',
    priority: 'medium',
    assignedAgents: m.assigned_agents,
    progress: m.status === 'completed' ? 100 : phaseProgress,
    createdAt: new Date(m.created_at),
    artifacts: m.artifacts.map(a => a.path || a.kind).filter(Boolean),
    phase: m.phase,
  };
}

const agentTypeIcons = {
  coder: Code2,
  analyst: Activity,
  tester: Terminal,
  reviewer: CheckCircle2,
};

const agentTypeColors = {
  coder: 'text-blue-400',
  analyst: 'text-purple-400',
  tester: 'text-green-400',
  reviewer: 'text-orange-400',
};

const statusColors = {
  idle: 'bg-gray-500',
  working: 'bg-blue-500 animate-pulse',
  completed: 'bg-green-500',
  error: 'bg-red-500',
};

const priorityColors = {
  low: 'bg-gray-500/20 text-gray-400',
  medium: 'bg-yellow-500/20 text-yellow-400',
  high: 'bg-orange-500/20 text-orange-400',
  critical: 'bg-red-500/20 text-red-400',
};

const artifactTypeIcons = {
  code: FileCode,
  test: Terminal,
  documentation: Database,
  config: Layers,
};

const categoryColors = {
  pattern: 'bg-blue-500/20 text-blue-400',
  solution: 'bg-green-500/20 text-green-400',
  error: 'bg-red-500/20 text-red-400',
  optimization: 'bg-purple-500/20 text-purple-400',
};

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  if (typeof window !== 'undefined' && window.__TAURI__) {
    try {
      return await window.__TAURI__.core.invoke<T>(cmd, args);
    } catch (e) {
      console.error(cmd, e);
      return null;
    }
  }
  return null;
}

export function AgentManagerPanel() {
  const [activeTab, setActiveTab] = useState('agents');
  const [missions, setMissions] = useState<Mission[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [knowledge, setKnowledge] = useState<KnowledgeEntry[]>([]);
  const [newGoal, setNewGoal] = useState('');
  const [starting, setStarting] = useState(false);

  const loadMissions = useCallback(async () => {
    const result = await invokeTauri<OrchestratorMission[]>('orchestrator_list_missions');
    if (result && Array.isArray(result)) {
      setMissions(result.map(toMission));
      // Collect artifacts from missions
      const allArtifacts: Artifact[] = result.flatMap((m, mi) =>
        m.artifacts.map((a, ai) => ({
          id: `${mi}-${ai}`,
          name: a.path?.split('/').pop() || a.kind,
          type: (a.kind === 'test' ? 'test' : a.kind === 'doc' ? 'documentation' : 'code') as Artifact['type'],
          path: a.path || `/${a.kind}`,
          createdAt: new Date(m.created_at),
          verified: m.status === 'completed',
          size: a.content ? `${(a.content.length / 1024).toFixed(1)} KB` : '—',
        }))
      );
      setArtifacts(allArtifacts);
    }
  }, []);

  const loadAgents = useCallback(async () => {
    // Try to load real agents from the swarm/agent system
    const result = await invokeTauri<Agent[]>('swarm_list_agents');
    if (result && Array.isArray(result) && result.length > 0) {
      setAgents(result);
    } else {
      // Default agent definitions (these represent capabilities, not fake data)
      setAgents([
        { id: 'coder', name: 'Code Generator', type: 'coder', status: 'idle', completedTasks: 0, successRate: 1.0 },
        { id: 'tester', name: 'Test Generator', type: 'tester', status: 'idle', completedTasks: 0, successRate: 1.0 },
        { id: 'reviewer', name: 'Code Reviewer', type: 'reviewer', status: 'idle', completedTasks: 0, successRate: 1.0 },
        { id: 'analyst', name: 'Analyzer', type: 'analyst', status: 'idle', completedTasks: 0, successRate: 1.0 },
      ]);
    }
  }, []);

  const loadKnowledge = useCallback(async () => {
    const result = await invokeTauri<KnowledgeEntry[]>('rag_list_knowledge');
    if (result && Array.isArray(result)) {
      setKnowledge(result);
    }
  }, []);

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      void loadMissions();
      void loadAgents();
      void loadKnowledge();
    }, 0);

    return () => clearTimeout(timeoutId);
  }, [loadMissions, loadAgents, loadKnowledge]);

  const handleStartMission = async () => {
    if (!newGoal.trim()) return;
    setStarting(true);
    const result = await invokeTauri<OrchestratorMission>('orchestrator_start_mission', { goal: newGoal.trim(), constraints: null });
    if (result) {
      setMissions(prev => [toMission(result), ...prev]);
      setNewGoal('');
    }
    setStarting(false);
  };

  const displayMissions = missions;

  return (
    <div className="h-full flex flex-col bg-[#0d1117] text-[#c9d1d9]">
      {/* Header */}
      <div className="p-4 border-b border-[#30363d] bg-linear-to-r from-[#161b22] to-[#0d1117]">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-linear-to-br from-purple-500/20 to-blue-500/20">
            <Rocket className="w-6 h-6 text-purple-400" />
          </div>
          <div>
            <h1 className="text-xl font-bold bg-linear-to-r from-purple-400 to-blue-400 bg-clip-text text-transparent">
              Mission Control
            </h1>
            <p className="text-xs text-[#8b949e]">Agent-powered development orchestration</p>
          </div>
        </div>
      </div>

      {/* Stats Overview */}
      <div className="grid grid-cols-4 gap-3 p-4 border-b border-[#30363d]">
        <Card className="bg-[#161b22] border-[#30363d]">
          <CardContent className="p-3">
            <div className="flex items-center gap-2">
              <Bot className="w-4 h-4 text-blue-400" />
              <span className="text-xs text-[#8b949e]">Active Agents</span>
            </div>
            <div className="text-2xl font-bold mt-1">{agents.filter(a => a.status === 'working').length}</div>
          </CardContent>
        </Card>
        <Card className="bg-[#161b22] border-[#30363d]">
          <CardContent className="p-3">
            <div className="flex items-center gap-2">
              <Target className="w-4 h-4 text-green-400" />
              <span className="text-xs text-[#8b949e]">Missions</span>
            </div>
            <div className="text-2xl font-bold mt-1">{displayMissions.length}</div>
          </CardContent>
        </Card>
        <Card className="bg-[#161b22] border-[#30363d]">
          <CardContent className="p-3">
            <div className="flex items-center gap-2">
              <FileCode className="w-4 h-4 text-orange-400" />
              <span className="text-xs text-[#8b949e]">Artifacts</span>
            </div>
            <div className="text-2xl font-bold mt-1">{artifacts.length}</div>
          </CardContent>
        </Card>
        <Card className="bg-[#161b22] border-[#30363d]">
          <CardContent className="p-3">
            <div className="flex items-center gap-2">
              <Brain className="w-4 h-4 text-purple-400" />
              <span className="text-xs text-[#8b949e]">Knowledge</span>
            </div>
            <div className="text-2xl font-bold mt-1">{knowledge.length}</div>
          </CardContent>
        </Card>
      </div>

      {/* Main Content */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
        <div className="px-4 pt-2 border-b border-[#30363d]">
          <TabsList className="bg-[#161b22] border-[#30363d]">
            <TabsTrigger value="agents" className="data-[state=active]:bg-[#21262d]">
              <Bot className="w-4 h-4 mr-2" /> Agents
            </TabsTrigger>
            <TabsTrigger value="missions" className="data-[state=active]:bg-[#21262d]">
              <Rocket className="w-4 h-4 mr-2" /> Missions
            </TabsTrigger>
            <TabsTrigger value="artifacts" className="data-[state=active]:bg-[#21262d]">
              <FileCode className="w-4 h-4 mr-2" /> Artifacts
            </TabsTrigger>
            <TabsTrigger value="knowledge" className="data-[state=active]:bg-[#21262d]">
              <Brain className="w-4 h-4 mr-2" /> Knowledge
            </TabsTrigger>
          </TabsList>
        </div>

        <ScrollArea className="flex-1">
          {/* Agents Tab */}
          <TabsContent value="agents" className="p-4 m-0">
            <div className="space-y-3">
              {agents.map((agent) => {
                const Icon = agentTypeIcons[agent.type];
                return (
                  <Card key={agent.id} className="bg-[#161b22] border-[#30363d] hover:border-[#58a6ff]/50 transition-colors">
                    <CardContent className="p-4">
                      <div className="flex items-start justify-between">
                        <div className="flex items-center gap-3">
                          <div className={`p-2 rounded-lg bg-[#21262d] ${agentTypeColors[agent.type]}`}>
                            <Icon className="w-5 h-5" />
                          </div>
                          <div>
                            <div className="flex items-center gap-2">
                              <h3 className="font-medium">{agent.name}</h3>
                              <div className={`w-2 h-2 rounded-full ${statusColors[agent.status]}`} />
                            </div>
                            <p className="text-xs text-[#8b949e] mt-0.5">
                              {agent.status === 'working' ? agent.currentTask : agent.status}
                            </p>
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          {agent.status === 'working' ? (
                            <Button size="sm" variant="outline" className="h-7 text-xs border-[#30363d] hover:bg-[#21262d]">
                              <Pause className="w-3 h-3 mr-1" /> Pause
                            </Button>
                          ) : (
                            <Button size="sm" variant="outline" className="h-7 text-xs border-[#30363d] hover:bg-[#21262d]">
                              <Play className="w-3 h-3 mr-1" /> Start
                            </Button>
                          )}
                        </div>
                      </div>
                      {agent.status === 'working' && agent.progress !== undefined && (
                        <div className="mt-3">
                          <div className="flex justify-between text-xs text-[#8b949e] mb-1">
                            <span>Progress</span>
                            <span>{agent.progress}%</span>
                          </div>
                          <Progress value={agent.progress} className="h-1.5 bg-[#21262d]" />
                        </div>
                      )}
                      <div className="flex items-center gap-4 mt-3 text-xs text-[#8b949e]">
                        <span className="flex items-center gap-1">
                          <CheckCircle2 className="w-3 h-3" /> {agent.completedTasks} completed
                        </span>
                        <span className="flex items-center gap-1">
                          <TrendingUp className="w-3 h-3" /> {(agent.successRate * 100).toFixed(0)}% success
                        </span>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          </TabsContent>

          {/* Missions Tab */}
          <TabsContent value="missions" className="p-4 m-0">
            <div className="flex gap-2 mb-4">
              <Input
                placeholder="Describe your goal (e.g. build auth module)"
                value={newGoal}
                onChange={(e) => setNewGoal(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleStartMission()}
                className="bg-[#161b22] border-[#30363d] text-[#c9d1d9]"
              />
              <Button onClick={handleStartMission} disabled={starting || !newGoal.trim()} size="sm">
                <Rocket className="w-4 h-4 mr-1" /> Start
              </Button>
            </div>
            <div className="space-y-3">
              {displayMissions.map((mission) => (
                <Card key={mission.id} className="bg-[#161b22] border-[#30363d] hover:border-[#58a6ff]/50 transition-colors">
                  <CardContent className="p-4">
                    <div className="flex items-start justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <Badge className={priorityColors[mission.priority]}>{mission.priority}</Badge>
                        <h3 className="font-medium">{mission.title}</h3>
                      </div>
                      <Badge variant="outline" className="border-[#30363d]">
                        {mission.phase ? `${mission.phase} • ` : ''}{mission.status.replace('_', ' ')}
                      </Badge>
                    </div>
                    <p className="text-xs text-[#8b949e] mb-3">{mission.description}</p>
                    <div className="flex items-center gap-4 mb-3">
                      <div className="flex items-center gap-1 text-xs text-[#8b949e]">
                        <Bot className="w-3 h-3" /> {mission.assignedAgents.length} agents
                      </div>
                      <div className="flex items-center gap-1 text-xs text-[#8b949e]">
                        <FileCode className="w-3 h-3" /> {mission.artifacts.length} artifacts
                      </div>
                      <div className="flex items-center gap-1 text-xs text-[#8b949e]">
                        <Clock className="w-3 h-3" /> {new Date(mission.createdAt).toLocaleDateString()}
                      </div>
                    </div>
                    {mission.status === 'in_progress' && (
                      <div>
                        <div className="flex justify-between text-xs text-[#8b949e] mb-1">
                          <span>Progress</span>
                          <span>{mission.progress}%</span>
                        </div>
                        <Progress value={mission.progress} className="h-1.5 bg-[#21262d]" />
                      </div>
                    )}
                  </CardContent>
                </Card>
              ))}
            </div>
          </TabsContent>

          {/* Artifacts Tab */}
          <TabsContent value="artifacts" className="p-4 m-0">
            <div className="space-y-2">
              {artifacts.length === 0 ? (
                <div className="text-center text-[#8b949e] py-8">No artifacts yet. Start a mission to generate artifacts.</div>
              ) : artifacts.map((artifact) => {
                const Icon = artifactTypeIcons[artifact.type];
                return (
                  <Card key={artifact.id} className="bg-[#161b22] border-[#30363d] hover:border-[#58a6ff]/50 transition-colors cursor-pointer">
                    <CardContent className="p-3">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <div className="p-2 rounded bg-[#21262d]">
                            <Icon className="w-4 h-4 text-[#8b949e]" />
                          </div>
                          <div>
                            <div className="flex items-center gap-2">
                              <h4 className="text-sm font-medium">{artifact.name}</h4>
                              {artifact.verified && (
                                <CheckCircle2 className="w-3 h-3 text-green-400" />
                              )}
                            </div>
                            <p className="text-xs text-[#8b949e]">{artifact.path}</p>
                          </div>
                        </div>
                        <div className="flex items-center gap-2 text-xs text-[#8b949e]">
                          <span>{artifact.size}</span>
                          <ChevronRight className="w-4 h-4" />
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          </TabsContent>

          {/* Knowledge Tab */}
          <TabsContent value="knowledge" className="p-4 m-0">
            <div className="space-y-2">
              {knowledge.length === 0 ? (
                <div className="text-center text-[#8b949e] py-8">No knowledge entries yet. The RAG system will populate this as you work.</div>
              ) : knowledge.map((entry) => (
                <Card key={entry.id} className="bg-[#161b22] border-[#30363d] hover:border-[#58a6ff]/50 transition-colors cursor-pointer">
                  <CardContent className="p-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <Badge className={categoryColors[entry.category]}>{entry.category}</Badge>
                        <div>
                          <h4 className="text-sm font-medium">{entry.title}</h4>
                          <p className="text-xs text-[#8b949e]">
                            Used {entry.usageCount} times | {(entry.relevance * 100).toFixed(0)}% relevant
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Sparkles className="w-3 h-3 text-yellow-400" />
                        <ChevronRight className="w-4 h-4 text-[#8b949e]" />
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          </TabsContent>
        </ScrollArea>
      </Tabs>

      {/* Footer Stats */}
      <div className="p-3 border-t border-[#30363d] bg-[#161b22]">
        <div className="flex items-center justify-between text-xs text-[#8b949e]">
          <div className="flex items-center gap-4">
            <span className="flex items-center gap-1">
              <Cpu className="w-3 h-3" /> CPU: 23%
            </span>
            <span className="flex items-center gap-1">
              <MemoryStick className="w-3 h-3" /> Memory: 4.2 GB
            </span>
          </div>
          <div className="flex items-center gap-4">
            <span className="flex items-center gap-1">
              <Zap className="w-3 h-3 text-yellow-400" /> AI Active
            </span>
            <span className="flex items-center gap-1">
              <GitBranch className="w-3 h-3" /> main
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
