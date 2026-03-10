import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import type { AgentPhase } from '../types.js';

const PHASE_CSS: Record<AgentPhase, string> = {
  idle:         'var(--geist-text-dim, #5a6a7a)',
  waiting:      'var(--geist-text-dim, #5a6a7a)',
  planning:     'var(--geist-active, #00d4ff)',
  thinking:     'var(--geist-active, #00d4ff)',
  reading:      '#0098b3',
  editing:      '#b47aff',
  testing:      '#4abb5a',
  tool_calling: '#ffb43a',
  error:        'var(--geist-error, #ff4a4a)',
};

@customElement('geist-dot')
export class GeistDot extends LitElement {
  @property() phase: AgentPhase = 'idle';

  static styles = css`
    :host {
      display: inline-block;
    }

    .dot {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      transition: background-color 0.3s ease, box-shadow 0.3s ease;
    }

    .dot.active {
      animation: pulse 2s ease-in-out infinite;
    }

    @keyframes pulse {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.5; }
    }
  `;

  render() {
    const color = PHASE_CSS[this.phase];
    const active = this.phase !== 'idle' && this.phase !== 'waiting';

    return html`
      <div
        class="dot ${active ? 'active' : ''}"
        style="background-color: ${color}; box-shadow: 0 0 6px ${color};"
      ></div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'geist-dot': GeistDot;
  }
}
