import { LitElement, html, css, svg } from 'lit';
import { customElement, property } from 'lit/decorators.js';

@customElement('geist-sparkline')
export class GeistSparkline extends LitElement {
  @property({ attribute: false }) values: number[] = [];
  @property({ type: Number }) width = 120;
  @property({ type: Number }) height = 24;
  @property() color = 'var(--geist-active, #00d4ff)';

  static styles = css`
    :host {
      display: inline-block;
    }

    svg {
      display: block;
    }

    polyline {
      fill: none;
      stroke-width: 1.5;
      stroke-linejoin: round;
      stroke-linecap: round;
    }
  `;

  private computePoints(): string {
    const vals = this.values;
    if (vals.length < 2) return '';

    const max = Math.max(...vals, 1);
    const padding = 2;
    const w = this.width - padding * 2;
    const h = this.height - padding * 2;
    const step = w / (vals.length - 1);

    return vals
      .map((v, i) => {
        const x = padding + i * step;
        const y = padding + h - (v / max) * h;
        return `${x},${y}`;
      })
      .join(' ');
  }

  render() {
    const points = this.computePoints();

    return html`
      <svg
        width="${this.width}"
        height="${this.height}"
        viewBox="0 0 ${this.width} ${this.height}"
      >
        ${points
          ? svg`<polyline
              points="${points}"
              stroke="${this.color}"
            />`
          : ''}
      </svg>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'geist-sparkline': GeistSparkline;
  }
}
