<script lang="ts">
	import Sigil from '$lib/components/sigil/Sigil.svelte';
	import GeistCenter from '$lib/components/sigil/GeistCenter.svelte';

</script>

<div class="landing">
	<!-- ═══ HERO ═══ -->
	<section class="hero">
		<div class="hero-grid" aria-hidden="true"></div>
		<div class="hero-glow" aria-hidden="true"></div>

		<div class="sigil-container">
			<Sigil size={650}>
				{#snippet center({ time })}
					<GeistCenter {time} />
				{/snippet}
			</Sigil>
		</div>

		<h1 class="title">geist.sh</h1>
		<p class="subtitle">adapt. compose. extend.</p>

		<div class="scroll-hint" aria-hidden="true">
			<span class="scroll-line"></span>
		</div>
	</section>

	<!-- ═══ THE PROBLEM ═══ -->
	<section class="section">
		<div class="terminal-block">
			<div class="terminal-line">
				<span class="prompt">$</span> describe --problem
			</div>
			<div class="terminal-output">
				<p>200,000 developers running privileged autonomous agents.</p>
				<p>Shell access. Filesystem. Network. Browser.</p>
				<p class="highlight">No governance layer.</p>
			</div>
		</div>
	</section>

	<!-- ═══ THE EQUATION ═══ -->
	<section class="section">
		<div class="equation">
			<span class="eq-term">geist</span>
			<span class="eq-op">+</span>
			<span class="eq-term">shell</span>
			<span class="eq-op">&rarr;</span>
			<span class="eq-term accent">gestalt</span>
		</div>
		<p class="eq-gloss">any agent + portable runtime &rarr; emergence</p>
	</section>

	<!-- ═══ FOUR-LAYER STACK ═══ -->
	<section class="section">
		<div class="stack">
			<div class="layer provided">
				<span class="layer-id">L4</span>
				<span class="layer-name">Agentic</span>
				<span class="layer-owner">geist</span>
			</div>
			<div class="layer yours">
				<span class="layer-id">L3</span>
				<span class="layer-name">Application</span>
				<span class="layer-owner">your code</span>
			</div>
			<div class="layer provided">
				<span class="layer-id">L2</span>
				<span class="layer-name">Gateway</span>
				<span class="layer-owner">shell</span>
			</div>
			<div class="layer provided">
				<span class="layer-id">L1</span>
				<span class="layer-name">Kernel</span>
				<span class="layer-owner">shell</span>
			</div>
		</div>
		<p class="stack-note">geist.sh = L1 + L2 + L4 &mdash; you bring L3</p>
	</section>

	<!-- ═══ ENTER ═══ -->
	<section class="section enter">
		<a href="/docs" class="terminal-enter">
			<span class="prompt">$</span> cd /docs <span class="cursor done">&#x2588;</span>
		</a>
		<div class="status-line">
			<span class="status-pair">
				<span class="label">domain</span>
				gestalt.mox.nexus
			</span>
			<span class="status-pair">
				<span class="label">status</span>
				<span class="draft">research &rarr; scaffold</span>
			</span>
		</div>
	</section>
</div>

<style>
	/* ════════════════════════════════════════════════════════
	   LANDING
	   ════════════════════════════════════════════════════════ */

	.landing {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
	}

	/* ── Hero ─────────────────────────────────────────────── */

	.hero {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		min-height: 100dvh;
		padding: var(--hud-space-8);
		overflow: hidden;
	}

	.hero-grid {
		position: absolute;
		inset: 0;
		background-image: linear-gradient(rgba(0, 255, 65, 0.018) 1px, transparent 1px),
			linear-gradient(90deg, rgba(0, 255, 65, 0.018) 1px, transparent 1px);
		background-size: 40px 40px;
		mask-image: radial-gradient(ellipse 60% 50% at 50% 50%, black 20%, transparent 70%);
		-webkit-mask-image: radial-gradient(ellipse 60% 50% at 50% 50%, black 20%, transparent 70%);
	}

	.hero-glow {
		position: absolute;
		width: 500px;
		height: 500px;
		border-radius: 50%;
		background: radial-gradient(circle, rgba(0, 255, 65, 0.06) 0%, transparent 70%);
		pointer-events: none;
	}

	.sigil-container {
		position: relative;
		z-index: 1;
		animation: sigil-enter 2s ease-out;
	}

	.title {
		position: relative;
		z-index: 1;
		margin-top: var(--hud-space-6);
		font-size: clamp(2rem, 6vw, 3.5rem);
		font-weight: var(--hud-weight-normal);
		color: var(--hud-accent);
		letter-spacing: 0.12em;
		text-shadow: 0 0 30px rgba(0, 255, 65, 0.4);
		animation: fade-in 1s ease-out 0.3s both;
	}

	.subtitle {
		position: relative;
		z-index: 1;
		margin-top: var(--hud-space-3);
		font-size: var(--hud-text-sm);
		color: var(--hud-text-faint);
		letter-spacing: 0.2em;
		animation: fade-in 1s ease-out 0.6s both;
	}

	.scroll-hint {
		position: absolute;
		bottom: var(--hud-space-8);
		display: flex;
		justify-content: center;
	}

	.scroll-line {
		display: block;
		width: 1px;
		height: 40px;
		background: linear-gradient(to bottom, var(--hud-text-faint), transparent);
		animation: scroll-pulse 2s ease-in-out infinite;
	}

	/* ── Sections ─────────────────────────────────────────── */

	.section {
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: var(--hud-space-10) var(--hud-space-8);
		max-width: 640px;
		margin: 0 auto;
		width: 100%;
	}

	/* ── Terminal block ───────────────────────────────────── */

	.terminal-block {
		width: 100%;
		border: 1px solid var(--hud-border-subtle);
		border-radius: var(--hud-radius);
		overflow: hidden;
	}

	.terminal-line {
		padding: var(--hud-space-3) var(--hud-space-4);
		font-size: var(--hud-text-sm);
		color: var(--hud-text);
		background: var(--hud-bg-surface);
		border-bottom: 1px solid var(--hud-border-subtle);
	}

	.prompt {
		color: var(--hud-accent);
		margin-right: var(--hud-space-2);
	}

	.terminal-output {
		padding: var(--hud-space-4);
		display: flex;
		flex-direction: column;
		gap: var(--hud-space-2);
	}

	.terminal-output p {
		font-size: var(--hud-text-sm);
		color: var(--hud-text-muted);
		line-height: var(--hud-leading-relaxed);
	}

	.highlight {
		color: var(--hud-accent) !important;
	}

	/* ── Equation ─────────────────────────────────────────── */

	.equation {
		display: flex;
		align-items: baseline;
		gap: var(--hud-space-3);
		flex-wrap: wrap;
		justify-content: center;
	}

	.eq-term {
		font-size: var(--hud-text-xl);
		color: var(--hud-text);
		letter-spacing: var(--hud-tracking-wide);
	}

	.eq-op {
		font-size: var(--hud-text-lg);
		color: var(--hud-text-faint);
	}

	.accent {
		color: var(--hud-accent);
		text-shadow: 0 0 12px rgba(0, 255, 65, 0.3);
	}

	.eq-gloss {
		margin-top: var(--hud-space-3);
		font-size: var(--hud-text-xs);
		color: var(--hud-text-faint);
		letter-spacing: var(--hud-tracking-wide);
	}

	/* ── Stack ────────────────────────────────────────────── */

	.stack {
		width: 100%;
		max-width: 480px;
		border: 1px solid var(--hud-border);
		border-radius: var(--hud-radius);
		overflow: hidden;
	}

	.layer {
		display: grid;
		grid-template-columns: 36px 1fr auto;
		gap: var(--hud-space-3);
		padding: var(--hud-space-3) var(--hud-space-4);
		align-items: center;
		border-bottom: 1px solid var(--hud-border-subtle);
		transition: background var(--hud-transition);
	}

	.layer:last-child {
		border-bottom: none;
	}

	.layer:hover {
		background: var(--hud-bg-hover);
	}

	.layer.provided {
		border-left: 2px solid var(--hud-accent);
	}

	.layer.yours {
		border-left: 2px dashed var(--hud-text-faint);
		background: var(--hud-bg-surface);
	}

	.layer-id {
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
		font-weight: var(--hud-weight-medium);
	}

	.layer-name {
		font-size: var(--hud-text-sm);
		color: var(--hud-text);
	}

	.yours .layer-name {
		color: var(--hud-text-muted);
	}

	.layer-owner {
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
		text-align: right;
	}

	.provided .layer-owner {
		color: var(--hud-accent);
		opacity: 0.6;
	}

	.stack-note {
		margin-top: var(--hud-space-4);
		font-size: var(--hud-text-xs);
		color: var(--hud-text-faint);
		letter-spacing: var(--hud-tracking-wide);
	}

	/* ── Enter ────────────────────────────────────────────── */

	.enter {
		padding-bottom: var(--hud-space-10);
	}

	.terminal-enter {
		display: inline-block;
		font-size: var(--hud-text-lg);
		color: var(--hud-text);
		text-decoration: none;
		padding: var(--hud-space-3) var(--hud-space-5);
		border: 1px solid var(--hud-border);
		border-radius: var(--hud-radius);
		transition: all 200ms ease-out;
	}

	.terminal-enter:hover {
		background: var(--hud-bg-hover);
		border-color: var(--hud-accent);
		text-decoration: none;
		text-shadow: 0 0 8px rgba(0, 255, 65, 0.3);
	}

	.status-line {
		margin-top: var(--hud-space-8);
		display: flex;
		gap: var(--hud-space-6);
		flex-wrap: wrap;
		justify-content: center;
	}

	.status-pair {
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
		display: flex;
		gap: var(--hud-space-2);
		align-items: center;
	}

	.label {
		text-transform: uppercase;
		letter-spacing: var(--hud-tracking-wider);
		opacity: 0.6;
	}

	.draft {
		color: var(--hud-status-draft);
	}

	/* ── Animations ───────────────────────────────────────── */

	@keyframes sigil-enter {
		from {
			opacity: 0;
			transform: scale(0.92);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}

	@keyframes fade-in {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes scroll-pulse {
		0%,
		100% {
			opacity: 0.2;
		}
		50% {
			opacity: 0.7;
		}
	}

	/* ── Responsive ───────────────────────────────────────── */

	@media (max-width: 640px) {
		.hero {
			padding: var(--hud-space-6);
		}

		.section {
			padding: var(--hud-space-8) var(--hud-space-4);
		}

		.equation {
			gap: var(--hud-space-2);
		}

		.eq-term {
			font-size: var(--hud-text-lg);
		}

		.stack {
			max-width: 100%;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.sigil-container,
		.title {
			animation: none;
		}

		.scroll-line {
			animation: none;
			opacity: 0.4;
		}
	}
</style>
