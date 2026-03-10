import type { ReactiveController, ReactiveControllerHost } from 'lit';
import type {
  HUDEvent,
  AgentPhase,
  TraceEvent,
  ProcessorMetric,
  ConnectionStatus,
  GeistConfig,
} from './types.js';
import { createWsTransport, type WsTransport } from './transport/ws.js';
import { createMockTransport, type MockTransport } from './transport/mock.js';

// ── Phase → visual mapping ──────────────────────────────────

const PHASE_COLORS: Record<AgentPhase, string> = {
  idle:         'var(--geist-text-dim, #5a6a7a)',
  waiting:      'var(--geist-text-dim, #5a6a7a)',
  planning:     'var(--geist-active, #00d4ff)',
  thinking:     'var(--geist-active, #00d4ff)',
  reading:      'color-mix(in srgb, var(--geist-active, #00d4ff) 60%, transparent)',
  editing:      '#b47aff',
  testing:      '#4abb5a',
  tool_calling: '#ffb43a',
  error:        'var(--geist-error, #ff4a4a)',
};

const BREATH_RATES: Record<AgentPhase, number> = {
  idle:         4.0,
  waiting:      3.5,
  planning:     2.5,
  thinking:     2.0,
  reading:      2.8,
  editing:      1.8,
  testing:      1.5,
  tool_calling: 1.2,
  error:        0.8,
};

const DEFAULT_MAX_TRACES = 200;

// ── Controller ──────────────────────────────────────────────

export class GeistController implements ReactiveController {
  readonly host: ReactiveControllerHost;

  // State
  status: ConnectionStatus = 'disconnected';
  phase: AgentPhase = 'idle';
  label = '';
  resource?: string;
  traces: TraceEvent[] = [];
  processors: ProcessorMetric[] = [];
  totalMs = 0;
  latencyHistory: number[] = [];
  uptimeS = 0;

  // Config
  private readonly maxTraces: number;
  private readonly maxLatencyPoints = 60;

  // Transport
  private wsTransport: WsTransport | null = null;
  private mockTransport: MockTransport | null = null;
  private readonly config: GeistConfig;

  // Derived
  get phaseColor(): string {
    return PHASE_COLORS[this.phase];
  }

  get breathRate(): number {
    return BREATH_RATES[this.phase];
  }

  constructor(host: ReactiveControllerHost, config: GeistConfig = {}) {
    this.host = host;
    this.config = config;
    this.maxTraces = config.maxTraces ?? DEFAULT_MAX_TRACES;
    host.addController(this);
  }

  hostConnected(): void {
    if (this.config.mock) {
      this.startMock();
    } else if (this.config.url) {
      this.startWs(this.config.url);
    }
  }

  hostDisconnected(): void {
    this.wsTransport?.disconnect();
    this.mockTransport?.stop();
    this.wsTransport = null;
    this.mockTransport = null;
  }

  // ── Transport setup ─────────────────────────────────────

  private startMock(): void {
    this.status = 'mock';
    this.mockTransport = createMockTransport((e) => this.handleEvent(e));
    this.mockTransport.start();
    this.host.requestUpdate();
  }

  private startWs(url: string): void {
    this.wsTransport = createWsTransport(
      url,
      (e) => this.handleEvent(e),
      (s) => {
        this.status = s;
        this.host.requestUpdate();
      },
      this.config.maxReconnectMs,
    );
    this.wsTransport.connect();
  }

  // ── Event handling ──────────────────────────────────────

  private handleEvent(event: HUDEvent): void {
    switch (event.type) {
      case 'agent_state':
        this.phase = event.phase;
        this.label = event.label;
        this.resource = event.resource;
        break;

      case 'trace':
        this.traces = [...this.traces.slice(-(this.maxTraces - 1)), event];
        break;

      case 'pipeline_metrics':
        this.processors = event.processors;
        this.totalMs = event.total_ms;
        this.latencyHistory = [
          ...this.latencyHistory.slice(-(this.maxLatencyPoints - 1)),
          event.total_ms,
        ];
        break;

      case 'heartbeat':
        this.uptimeS = event.uptime_s;
        break;
    }

    this.host.requestUpdate();
  }

  // ── Public API ──────────────────────────────────────────

  send(msg: unknown): void {
    this.wsTransport?.send(msg);
  }
}
