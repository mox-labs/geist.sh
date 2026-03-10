import { LitElement, html, css, nothing } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import type { TraceEvent, TraceLevel } from '../types.js';

const LEVEL_ICONS: Record<TraceLevel, string> = {
  info:        '\u2022',      // bullet
  tool_call:   '\u25B6',      // right triangle
  tool_result: '\u25C0',      // left triangle
  thinking:    '\u25CB',      // circle outline
  error:       '\u2716',      // heavy X
};

const LEVEL_COLORS: Record<TraceLevel, string> = {
  info:        'var(--geist-text-dim, #5a6a7a)',
  tool_call:   '#ffb43a',
  tool_result: '#4abb5a',
  thinking:    'var(--geist-active, #00d4ff)',
  error:       'var(--geist-error, #ff4a4a)',
};

@customElement('geist-trace')
export class GeistTrace extends LitElement {
  @property({ attribute: false }) traces: TraceEvent[] = [];

  static styles = css`
    :host {
      display: block;
      height: 100%;
      overflow-y: auto;
      font-family: var(--geist-font, var(--hud-font-mono, 'JetBrains Mono', monospace));
      font-size: 0.75rem;
      padding: 0.5rem;
    }

    .entry {
      display: flex;
      gap: 0.5em;
      padding: 0.2em 0;
      animation: fadeIn 0.2s ease;
      line-height: 1.4;
    }

    @keyframes fadeIn {
      from { opacity: 0; transform: translateY(4px); }
      to   { opacity: 1; transform: translateY(0); }
    }

    .icon {
      flex-shrink: 0;
      width: 1.2em;
      text-align: center;
    }

    .summary {
      color: var(--geist-text, var(--hud-text, #e0e0e0));
    }

    details {
      cursor: pointer;
    }

    details > summary {
      list-style: none;
    }

    details > summary::-webkit-details-marker {
      display: none;
    }

    .detail, .deep {
      margin-top: 0.2em;
      padding-left: 1.7em;
      color: var(--geist-text-dim, var(--hud-text-muted, #5a6a7a));
      white-space: pre-wrap;
      font-size: 0.7rem;
    }

    .deep {
      opacity: 0.6;
    }
  `;

  protected updated(): void {
    const container = this.shadowRoot;
    if (container) {
      const entries = container.querySelectorAll('.entry');
      const last = entries[entries.length - 1];
      last?.scrollIntoView({ behavior: 'smooth', block: 'end' });
    }
  }

  private renderEntry(trace: TraceEvent, idx: number) {
    const icon = LEVEL_ICONS[trace.level];
    const color = LEVEL_COLORS[trace.level];
    const hasDisclosure = trace.detail || trace.deep;

    const content = hasDisclosure
      ? html`
          <details>
            <summary>
              <span class="icon" style="color: ${color}">${icon}</span>
              <span class="summary">${trace.summary}</span>
            </summary>
            ${trace.detail ? html`<div class="detail">${trace.detail}</div>` : nothing}
            ${trace.deep ? html`<div class="deep">${trace.deep}</div>` : nothing}
          </details>
        `
      : html`
          <span class="icon" style="color: ${color}">${icon}</span>
          <span class="summary">${trace.summary}</span>
        `;

    return html`<div class="entry">${content}</div>`;
  }

  render() {
    if (!this.traces.length) return nothing;

    return html`
      ${this.traces.map((t, i) => this.renderEntry(t, i))}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'geist-trace': GeistTrace;
  }
}
