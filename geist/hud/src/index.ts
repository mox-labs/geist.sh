// Register all custom elements (side-effect imports)
import './components/geist-projection.js';

// Re-export for programmatic use
export { GeistController } from './controller.js';
export { geistContext } from './context.js';
export type {
  HUDEvent,
  AgentStateEvent,
  TraceEvent,
  PipelineMetricsEvent,
  HeartbeatEvent,
  ProcessorMetric,
  AgentPhase,
  TraceLevel,
  ConnectionStatus,
  GeistConfig,
} from './types.js';
