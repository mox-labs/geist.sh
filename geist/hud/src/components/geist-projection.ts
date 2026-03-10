import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { GeistController } from '../controller.js';

// Import child components (side-effect: registers custom elements)
import './geist-breath.js';
import './geist-dot.js';
import './geist-phase.js';
import './geist-sparkline.js';
import './geist-trace.js';
import './geist-pipeline.js';

@customElement('geist-projection')
export class GeistProjection extends LitElement {
  @property() ws?: string;
  @property({ type: Boolean }) mock = false;

  controller!: GeistController;

  static styles = css`
    :host {
      display: flex;
      flex-direction: column;
      height: 100%;
      position: relative;
      font-family: var(--geist-font, var(--hud-font-mono, 'JetBrains Mono', monospace));
      color: var(--geist-text, var(--hud-text, #e0e0e0));
      background: var(--geist-bg, var(--hud-bg-surface, #0a0a10));
      overflow: hidden;

      /* Internal tokens with host-cascadable fallbacks */
      --geist-active:   var(--hud-cyan, #00d4ff);
      --geist-error:    var(--hud-error, #ff4a4a);
      --geist-text:     var(--hud-text, #e0e0e0);
      --geist-text-dim: var(--hud-text-muted, #5a6a7a);
      --geist-bg:       var(--hud-bg-surface, #0a0a10);
      --geist-font:     var(--hud-font-mono, 'JetBrains Mono', monospace);
    }

    header {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.75rem 1rem;
      z-index: 1;
      border-bottom: 1px solid color-mix(in srgb, var(--geist-text-dim) 20%, transparent);
    }

    main {
      flex: 1;
      overflow: hidden;
      z-index: 1;
    }

    footer {
      border-top: 1px solid color-mix(in srgb, var(--geist-text-dim) 20%, transparent);
      z-index: 1;
    }

    geist-phase {
      flex: 1;
      min-width: 0;
    }
  `;

  connectedCallback(): void {
    this.controller = new GeistController(this, {
      url: this.ws,
      mock: this.mock,
    });
    super.connectedCallback();
  }

  render() {
    const c = this.controller;

    return html`
      <geist-breath
        .phase=${c.phase}
        .color=${c.phaseColor}
        .rate=${c.breathRate}
      ></geist-breath>

      <header>
        <geist-dot .phase=${c.phase}></geist-dot>
        <geist-phase
          .label=${c.label || c.phase}
          .resource=${c.resource}
        ></geist-phase>
        <geist-sparkline
          .values=${c.latencyHistory}
        ></geist-sparkline>
      </header>

      <main>
        <geist-trace .traces=${c.traces}></geist-trace>
      </main>

      <footer>
        <geist-pipeline .processors=${c.processors}></geist-pipeline>
      </footer>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'geist-projection': GeistProjection;
  }
}
