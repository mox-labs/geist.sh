import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';

@customElement('geist-phase')
export class GeistPhase extends LitElement {
  @property() label = '';
  @property() resource?: string;

  static styles = css`
    :host {
      display: inline-flex;
      align-items: baseline;
      gap: 0.5em;
      font-family: var(--geist-font, var(--hud-font-mono, 'JetBrains Mono', monospace));
      overflow: hidden;
    }

    .label {
      font-size: 0.85rem;
      color: var(--geist-text, var(--hud-text, #e0e0e0));
      white-space: nowrap;
      transition: opacity 0.2s ease;
    }

    .resource {
      font-size: 0.75rem;
      color: var(--geist-text-dim, var(--hud-text-muted, #5a6a7a));
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  `;

  render() {
    return html`
      <span class="label">${this.label}</span>
      ${this.resource
        ? html`<span class="resource">${this.resource}</span>`
        : ''}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'geist-phase': GeistPhase;
  }
}
