<script lang="ts">
	import { allDocs } from 'content-collections';

	const sorted = $derived(
		[...allDocs].sort((a, b) => (a.order ?? 99) - (b.order ?? 99))
	);
</script>

<div class="docs-index">
	<h1 class="page-title">Documentation</h1>
	<p class="page-description">Architecture, design decisions, and technical reference for gestalt.</p>

	<nav class="doc-list">
		{#each sorted as doc}
			<a href="/docs/{doc.slug}" class="doc-item">
				<span class="doc-item-title">{doc.title}</span>
				{#if doc.description}
					<span class="doc-item-desc">{doc.description}</span>
				{/if}
			</a>
		{/each}
	</nav>

	<div class="doc-section">
		<h2 class="section-title">Architecture Decision Records</h2>
		<a href="/docs/decisions" class="doc-item">
			<span class="doc-item-title">ADRs</span>
			<span class="doc-item-desc">Key technical decisions and their rationale</span>
		</a>
	</div>
</div>

<style>
	.docs-index {
		padding: var(--hud-space-8) var(--hud-space-5);
		max-width: 720px;
	}

	.page-title {
		font-size: var(--hud-text-xl);
		font-weight: var(--hud-weight-medium);
		color: var(--hud-text);
	}

	.page-description {
		font-size: var(--hud-text-sm);
		color: var(--hud-text-secondary);
		margin-top: var(--hud-space-2);
		margin-bottom: var(--hud-space-8);
	}

	.doc-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.doc-item {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: var(--hud-space-3) var(--hud-space-4);
		border-radius: var(--hud-radius);
		text-decoration: none;
		transition: all var(--hud-transition);
	}

	.doc-item:hover {
		background: var(--hud-bg-hover);
		text-decoration: none;
	}

	.doc-item-title {
		font-size: var(--hud-text-sm);
		color: var(--hud-text);
	}

	.doc-item-desc {
		font-size: var(--hud-text-xs);
		color: var(--hud-text-muted);
	}

	.doc-section {
		margin-top: var(--hud-space-8);
		padding-top: var(--hud-space-5);
		border-top: 1px solid var(--hud-border-subtle);
	}

	.section-title {
		font-size: var(--hud-text-sm);
		font-weight: var(--hud-weight-medium);
		color: var(--hud-text-secondary);
		margin-bottom: var(--hud-space-3);
	}
</style>
