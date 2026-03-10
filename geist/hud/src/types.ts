// ── Wire protocol ────────────────────────────────────────

/** Discriminated union of all HUD events from geist-edge. */
export type HUDEvent =
  | AgentStateEvent
  | TraceEvent
  | PipelineMetricsEvent
  | HeartbeatEvent;

export interface AgentStateEvent {
  type: 'agent_state';
  timestamp: number;
  phase: AgentPhase;
  label: string;
  resource?: string;
}

export interface TraceEvent {
  type: 'trace';
  timestamp: number;
  level: TraceLevel;
  summary: string;
  detail?: string;
  deep?: string;
}

export interface PipelineMetricsEvent {
  type: 'pipeline_metrics';
  timestamp: number;
  processors: ProcessorMetric[];
  total_ms: number;
}

export interface ProcessorMetric {
  name: string;
  type_url: string;
  latency_ms: number;
  last_outcome: 'continue' | 'respond' | 'error';
}

export interface HeartbeatEvent {
  type: 'heartbeat';
  timestamp: number;
  uptime_s: number;
}

// ── Enums ────────────────────────────────────────────────

export type AgentPhase =
  | 'idle'
  | 'planning'
  | 'reading'
  | 'editing'
  | 'testing'
  | 'thinking'
  | 'tool_calling'
  | 'waiting'
  | 'error';

export type TraceLevel =
  | 'info'
  | 'tool_call'
  | 'tool_result'
  | 'thinking'
  | 'error';

export type ConnectionStatus =
  | 'connecting'
  | 'connected'
  | 'disconnected'
  | 'mock';

// ── Config ───────────────────────────────────────────────

export interface GeistConfig {
  /** WebSocket URL (ws:// or wss://). Omit for mock. */
  url?: string;
  /** Enable mock transport. */
  mock?: boolean;
  /** Max trace events to retain. Default 200. */
  maxTraces?: number;
  /** Reconnect interval cap in ms. Default 10000. */
  maxReconnectMs?: number;
}
