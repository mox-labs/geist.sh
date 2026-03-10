import type { HUDEvent, ConnectionStatus } from '../types.js';

const VALID_TYPES = new Set(['agent_state', 'trace', 'pipeline_metrics', 'heartbeat']);

export interface WsTransport {
  connect(): void;
  disconnect(): void;
  send(msg: unknown): void;
  readonly status: ConnectionStatus;
}

export function createWsTransport(
  url: string,
  onEvent: (e: HUDEvent) => void,
  onStatusChange: (s: ConnectionStatus) => void,
  maxReconnectMs = 10_000,
): WsTransport {
  let ws: WebSocket | null = null;
  let reconnectDelay = 1000;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let intentionalClose = false;
  let currentStatus: ConnectionStatus = 'disconnected';

  function setStatus(s: ConnectionStatus) {
    currentStatus = s;
    onStatusChange(s);
  }

  function tryConnect() {
    if (typeof WebSocket === 'undefined') return;

    setStatus('connecting');
    ws = new WebSocket(url);

    ws.onopen = () => {
      reconnectDelay = 1000;
      setStatus('connected');
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data && typeof data.type === 'string' && VALID_TYPES.has(data.type)) {
          onEvent(data as HUDEvent);
        }
      } catch {
        // Ignore malformed messages
      }
    };

    ws.onclose = () => {
      ws = null;
      if (!intentionalClose) {
        setStatus('disconnected');
        scheduleReconnect();
      }
    };

    ws.onerror = () => {
      // onclose will fire after onerror
    };
  }

  function scheduleReconnect() {
    if (intentionalClose) return;
    reconnectTimer = setTimeout(() => {
      reconnectDelay = Math.min(reconnectDelay * 2, maxReconnectMs);
      tryConnect();
    }, reconnectDelay);
  }

  return {
    connect() {
      intentionalClose = false;
      tryConnect();
    },

    disconnect() {
      intentionalClose = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      if (ws) {
        ws.close();
        ws = null;
      }
      setStatus('disconnected');
    },

    send(msg: unknown) {
      if (ws?.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(msg));
      }
    },

    get status() {
      return currentStatus;
    },
  };
}
