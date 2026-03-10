import { LitElement, html, css, svg, nothing } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import type { ProcessorMetric } from '../types.js';

const NODE_W = 100;
const NODE_H = 32;
const GAP = 40;
const PADDING = 16;

const OUTCOME_COLORS: Record<string, string> = {
  continue: 'var(--geist-active, #00d4ff)',
  respond:  '#4abb5a',
  error:    'var(--geist-error, #ff4a4a)',
};

@customElement('geist-pipeline')
export class GeistPipeline extends LitElement {
  @property({ attribute: false }) processors: ProcessorMetric[] = [];

  static styles = css`
    :host {
      display: block;
      overflow-x: auto;
      padding: 0.5rem;
    }

    svg {
      display: block;
    }

    text {
      font-family: var(--geist-font, var(--hud-font-mono, 'JetBrains Mono', monospace));
      font-size: 10px;
      fill: var(--geist-text, var(--hud-text, #e0e0e0));
    }

    .latency {
      font-size: 8px;
      fill: var(--geist-text-dim, var(--hud-text-muted, #5a6a7a));
    }

    rect {
      fill: var(--geist-bg, var(--hud-bg-surface, #0a0a10));
      rx: 4;
      ry: 4;
    }

    .edge {
      fill: none;
      stroke-width: 1.5;
      stroke-dasharray: 6 4;
    }

    @keyframes flow {
      to { stroke-dashoffset: -20; }
    }

    .edge.flowing {
      animation: flow 1s linear infinite;
    }
  `;

  private renderNode(proc: ProcessorMetric, x: number, y: number) {
    const color = OUTCOME_COLORS[proc.last_outcome] ?? OUTCOME_COLORS.continue;
    const shortName = proc.name.length > 12
      ? proc.name.slice(0, 11) + '\u2026'
      : proc.name;

    return svg`
      <rect
        x="${x}" y="${y}"
        width="${NODE_W}" height="${NODE_H}"
        stroke="${color}" stroke-width="1.5"
      />
      <text x="${x + NODE_W / 2}" y="${y + 14}" text-anchor="middle" dominant-baseline="middle">
        ${shortName}
      </text>
      <text class="latency" x="${x + NODE_W / 2}" y="${y + 26}" text-anchor="middle">
        ${proc.latency_ms}ms
      </text>
    `;
  }

  private renderEdge(x1: number, x2: number, y: number, flowing: boolean) {
    const color = 'var(--geist-text-dim, #5a6a7a)';
    return svg`
      <line
        class="edge ${flowing ? 'flowing' : ''}"
        x1="${x1}" y1="${y}"
        x2="${x2}" y2="${y}"
        stroke="${color}"
      />
    `;
  }

  render() {
    if (!this.processors.length) return nothing;

    const procs = this.processors;
    const totalW = PADDING * 2 + procs.length * NODE_W + (procs.length - 1) * GAP;
    const totalH = PADDING * 2 + NODE_H;
    const cy = PADDING + NODE_H / 2;

    return html`
      <svg
        width="${totalW}"
        height="${totalH}"
        viewBox="0 0 ${totalW} ${totalH}"
      >
        ${procs.map((p, i) => {
          const x = PADDING + i * (NODE_W + GAP);

          return svg`
            ${i > 0
              ? this.renderEdge(
                  PADDING + (i - 1) * (NODE_W + GAP) + NODE_W,
                  x,
                  cy,
                  p.last_outcome === 'continue',
                )
              : nothing}
            ${this.renderNode(p, x, PADDING)}
          `;
        })}
      </svg>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'geist-pipeline': GeistPipeline;
  }
}
