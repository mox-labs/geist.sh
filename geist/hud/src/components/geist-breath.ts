import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import type { AgentPhase } from '../types.js';

@customElement('geist-breath')
export class GeistBreath extends LitElement {
  @property() phase: AgentPhase = 'idle';
  @property() color = '#00d4ff';
  @property({ type: Number }) rate = 4.0;

  private rafId = 0;
  private startTime = 0;

  static styles = css`
    :host {
      display: block;
      position: absolute;
      inset: 0;
      pointer-events: none;
      z-index: 0;
    }

    .glow {
      width: 100%;
      height: 100%;
      will-change: opacity;
    }
  `;

  connectedCallback(): void {
    super.connectedCallback();
    this.startTime = performance.now();
    this.tick();
  }

  disconnectedCallback(): void {
    super.disconnectedCallback();
    cancelAnimationFrame(this.rafId);
  }

  private tick = (): void => {
    const elapsed = (performance.now() - this.startTime) / 1000;
    const frequency = (2 * Math.PI) / this.rate;
    // Sinusoidal 0.03 → 0.15 for subtle ambient glow
    const opacity = 0.03 + 0.12 * (0.5 + 0.5 * Math.sin(elapsed * frequency));

    const el = this.shadowRoot?.querySelector('.glow') as HTMLElement | null;
    if (el) {
      el.style.opacity = String(opacity);
      el.style.background = `radial-gradient(ellipse at 50% 40%, ${this.color}, transparent 70%)`;
    }

    this.rafId = requestAnimationFrame(this.tick);
  };

  render() {
    return html`<div class="glow"></div>`;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'geist-breath': GeistBreath;
  }
}
