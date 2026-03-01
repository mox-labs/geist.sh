<script lang="ts">
	import { allDecisions } from 'content-collections';

	const sorted = $derived(
		[...allDecisions].sort((a, b) => a.title.localeCompare(b.title))
	);
</script>

<svelte:head>
	<title>gestalt | decisions</title>
</svelte:head>

<div class="decisions-index">
	<h1 class="page-title">Architecture Decision Records</h1>
	<p class="page-description">Key technical decisions and their rationale.</p>

	<nav class="decision-list">
		{#each sorted as decision}
			<a href="/docs/decisions/{decision.slug}" class="decision-item">
				<div class="decision-header">
					<span class="decision-title">{decision.title}</span>
					{#if decision.status}
						<span
							class="badge"
							class:accepted={decision.status === 'accepted'}
							class:proposed={decision.status === 'proposed'}
							class:superseded={decision.status === 'superseded'}
						>
							{decision.status}
						</span>
					{/if}
				</div>
				<span class="decision-date">{decision.date}</span>
			</a>
		{/each}
	</nav>
</div>

<style>
	.decisions-index {
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

	.decision-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.decision-item {
		display: flex;
		flex-direction: column;
		gap: var(--hud-space-1);
		padding: var(--hud-space-3) var(--hud-space-4);
		border-radius: var(--hud-radius);
		text-decoration: none;
		transition: all var(--hud-transition);
	}

	.decision-item:hover {
		background: var(--hud-bg-hover);
		text-decoration: none;
	}

	.decision-header {
		display: flex;
		align-items: center;
		gap: var(--hud-space-3);
	}

	.decision-title {
		font-size: var(--hud-text-sm);
		color: var(--hud-text);
	}

	.badge {
		font-size: var(--hud-text-3xs);
		text-transform: uppercase;
		letter-spacing: var(--hud-tracking-wide);
		padding: 1px var(--hud-space-2);
		border-radius: var(--hud-radius);
		border: 1px solid;
	}

	.badge.accepted {
		color: var(--hud-green);
		border-color: var(--hud-green-border);
		background: var(--hud-green-soft);
	}

	.badge.proposed {
		color: var(--hud-amber);
		border-color: var(--hud-amber-border);
		background: var(--hud-amber-soft);
	}

	.badge.superseded {
		color: var(--hud-red);
		border-color: var(--hud-red-border);
		background: var(--hud-red-soft);
	}

	.decision-date {
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
	}
</style>
