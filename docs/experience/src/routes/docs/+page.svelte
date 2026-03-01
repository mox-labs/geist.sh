<script lang="ts">
	import { allDocs } from 'content-collections';

	const sorted = $derived(
		[...allDocs].sort((a, b) => (a.order ?? 99) - (b.order ?? 99))
	);

	const overviewDocs = $derived(sorted.filter(d => d.section === 'overview'));
	const archDocs = $derived(sorted.filter(d => d.section === 'architecture'));
</script>

<svelte:head>
	<title>gestalt | docs</title>
</svelte:head>

<div class="fs-listing">
	<div class="terminal-header">
		<span class="prompt">$</span> ls /docs
	</div>

	<div class="fs-section">
		<span class="fs-dir">overview/</span>
		{#each overviewDocs as doc}
			<a href="/docs/{doc.slug}" class="fs-entry">
				<span class="fs-name">{doc.slug}</span>
				<span class="fs-desc">{doc.description ?? doc.title}</span>
			</a>
		{/each}
	</div>

	<div class="fs-section">
		<span class="fs-dir">concepts/</span>
		{#each archDocs as doc}
			<a href="/docs/{doc.slug}" class="fs-entry">
				<span class="fs-name">{doc.slug}</span>
				<span class="fs-desc">{doc.description ?? doc.title}</span>
			</a>
		{/each}
	</div>

	<div class="fs-section">
		<span class="fs-dir">reference/</span>
		<a href="/docs/status" class="fs-entry">
			<span class="fs-name">status</span>
			<span class="fs-desc">Live phase tracker</span>
		</a>
		<a href="/docs/decisions" class="fs-entry">
			<span class="fs-name">decisions/</span>
			<span class="fs-desc">Architecture decision records</span>
		</a>
	</div>
</div>

<style>
	.fs-listing {
		padding: var(--hud-space-8) var(--hud-space-5);
		max-width: 720px;
	}

	.terminal-header {
		font-size: var(--hud-text-lg);
		color: var(--hud-text);
		margin-bottom: var(--hud-space-8);
	}

	.prompt {
		color: var(--hud-accent);
		margin-right: var(--hud-space-2);
	}

	.fs-section {
		margin-bottom: var(--hud-space-6);
		display: flex;
		flex-direction: column;
	}

	.fs-dir {
		font-size: var(--hud-text-sm);
		color: var(--hud-accent);
		font-weight: var(--hud-weight-medium);
		padding: var(--hud-space-1) 0;
		opacity: 0.7;
	}

	.fs-entry {
		display: grid;
		grid-template-columns: 240px 1fr;
		gap: var(--hud-space-4);
		padding: var(--hud-space-1) var(--hud-space-4);
		text-decoration: none;
		border-radius: var(--hud-radius);
		transition: background var(--hud-transition);
	}

	.fs-entry:hover {
		background: var(--hud-bg-hover);
		text-decoration: none;
	}

	.fs-name {
		font-size: var(--hud-text-sm);
		color: var(--hud-text);
	}

	.fs-desc {
		font-size: var(--hud-text-sm);
		color: var(--hud-text-muted);
	}

	@media (max-width: 640px) {
		.fs-entry {
			grid-template-columns: 1fr;
			gap: 2px;
		}
	}
</style>
