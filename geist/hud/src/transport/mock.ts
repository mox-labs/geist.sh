import type { HUDEvent, AgentPhase, TraceLevel } from '../types.js';

/** Phase durations in ms [min, max]. */
const PHASE_TIMING: Record<AgentPhase, [number, number]> = {
  idle:         [3000, 6000],
  planning:     [2000, 5000],
  reading:      [1500, 4000],
  editing:      [3000, 8000],
  testing:      [2000, 5000],
  thinking:     [1000, 3000],
  tool_calling: [500,  2000],
  waiting:      [2000, 4000],
  error:        [1000, 3000],
};

/** Normal phase cycle. */
const CYCLE: AgentPhase[] = [
  'idle', 'planning', 'reading', 'editing', 'testing', 'idle',
];

const SAMPLE_FILES = [
  'src/auth.rs', 'src/main.rs', 'lib/config.ts', 'tests/integration.rs',
  'src/pipeline.rs', 'src/processor.rs', 'Cargo.toml', 'README.md',
];

const SAMPLE_TOOLS = [
  'read_file', 'write_file', 'run_tests', 'search_code',
  'list_dir', 'execute_command', 'analyze_code',
];

const SAMPLE_PROCESSORS: Array<{ name: string; type_url: string }> = [
  { name: 'access-control', type_url: 'mox.geist.processors.v1.AccessControl' },
  { name: 'rate-limiter',   type_url: 'mox.geist.processors.v1.RateLimiter' },
  { name: 'otel-export',    type_url: 'mox.geist.processors.v1.OTelExport' },
];

function rand(min: number, max: number): number {
  return min + Math.random() * (max - min);
}

function pick<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

export interface MockTransport {
  start(): void;
  stop(): void;
}

export function createMockTransport(onEvent: (e: HUDEvent) => void): MockTransport {
  let running = false;
  let phaseTimer: ReturnType<typeof setTimeout> | null = null;
  let traceInterval: ReturnType<typeof setInterval> | null = null;
  let metricsInterval: ReturnType<typeof setInterval> | null = null;
  let heartbeatInterval: ReturnType<typeof setInterval> | null = null;
  let cycleIndex = 0;
  let startTime = 0;

  function now(): number {
    return Date.now();
  }

  function emitPhase(phase: AgentPhase) {
    const file = phase === 'reading' || phase === 'editing' ? pick(SAMPLE_FILES) : undefined;
    const labels: Record<AgentPhase, string> = {
      idle: 'Idle',
      planning: 'Planning approach',
      reading: `Reading ${file}`,
      editing: `Editing ${file}`,
      testing: 'Running tests',
      thinking: 'Analyzing results',
      tool_calling: `Calling ${pick(SAMPLE_TOOLS)}`,
      waiting: 'Waiting for input',
      error: 'Error encountered',
    };

    onEvent({
      type: 'agent_state',
      timestamp: now(),
      phase,
      label: labels[phase],
      resource: file,
    });
  }

  function emitTrace(phase: AgentPhase) {
    const levelMap: Partial<Record<AgentPhase, TraceLevel>> = {
      planning: 'thinking',
      reading: 'tool_call',
      editing: 'tool_call',
      testing: 'tool_result',
      thinking: 'thinking',
      tool_calling: 'tool_call',
    };

    const level = levelMap[phase] ?? 'info';
    const file = pick(SAMPLE_FILES);
    const tool = pick(SAMPLE_TOOLS);

    const summaries: Record<TraceLevel, string> = {
      info: `Phase: ${phase}`,
      tool_call: `${tool}(${file})`,
      tool_result: `${tool} completed — ${Math.floor(rand(1, 20))} results`,
      thinking: `Considering ${pick(['approach A', 'edge cases', 'error handling', 'test coverage', 'refactor options'])}`,
      error: `Error in ${file}: ${pick(['type mismatch', 'missing field', 'timeout', 'connection refused'])}`,
    };

    onEvent({
      type: 'trace',
      timestamp: now(),
      level,
      summary: summaries[level],
      detail: level === 'tool_call' ? `Executing ${tool} on ${file}` : undefined,
      deep: level === 'error' ? `stack trace:\n  at process_request (${file}:42)\n  at pipeline.run (pipeline.rs:108)` : undefined,
    });
  }

  function emitMetrics() {
    onEvent({
      type: 'pipeline_metrics',
      timestamp: now(),
      processors: SAMPLE_PROCESSORS.map(p => ({
        ...p,
        latency_ms: Math.round(rand(0.5, 15)),
        last_outcome: Math.random() > 0.05 ? 'continue' as const : 'respond' as const,
      })),
      total_ms: Math.round(rand(2, 30)),
    });
  }

  function emitHeartbeat() {
    onEvent({
      type: 'heartbeat',
      timestamp: now(),
      uptime_s: Math.round((now() - startTime) / 1000),
    });
  }

  function scheduleNextPhase() {
    if (!running) return;

    const phase = CYCLE[cycleIndex % CYCLE.length];
    const [min, max] = PHASE_TIMING[phase];
    const duration = rand(min, max);

    emitPhase(phase);

    phaseTimer = setTimeout(() => {
      cycleIndex++;
      scheduleNextPhase();
    }, duration);
  }

  return {
    start() {
      if (running) return;
      running = true;
      startTime = now();
      cycleIndex = 0;

      scheduleNextPhase();

      // Trace events during active phases (every 800-2000ms)
      traceInterval = setInterval(() => {
        const phase = CYCLE[cycleIndex % CYCLE.length];
        if (phase !== 'idle') {
          emitTrace(phase);
        }
      }, 1200);

      // Pipeline metrics every 3s
      metricsInterval = setInterval(emitMetrics, 3000);

      // Heartbeat every 5s
      heartbeatInterval = setInterval(emitHeartbeat, 5000);

      // Initial heartbeat
      emitHeartbeat();
      emitMetrics();
    },

    stop() {
      running = false;
      if (phaseTimer) clearTimeout(phaseTimer);
      if (traceInterval) clearInterval(traceInterval);
      if (metricsInterval) clearInterval(metricsInterval);
      if (heartbeatInterval) clearInterval(heartbeatInterval);
    },
  };
}
