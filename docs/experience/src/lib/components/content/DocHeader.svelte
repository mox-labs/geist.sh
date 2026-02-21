<script lang="ts">
	interface Props {
		title: string;
		description?: string;
		status?: string;
		date?: string;
	}

	let { title, description, status, date }: Props = $props();
</script>

<header class="doc-header">
	<h1 class="doc-title">{title}</h1>

	{#if description}
		<p class="doc-description">{description}</p>
	{/if}

	{#if status || date}
		<div class="doc-meta">
			{#if status}
				<span class="badge" class:accepted={status === 'accepted'} class:proposed={status === 'proposed'} class:superseded={status === 'superseded'} class:deprecated={status === 'deprecated'}>
					{status}
				</span>
			{/if}
			{#if date}
				<span class="doc-date">{date}</span>
			{/if}
		</div>
	{/if}
</header>

<style>
	.doc-header {
		padding: var(--hud-space-8) var(--hud-space-5) var(--hud-space-5);
		border-bottom: 1px solid var(--hud-border-subtle);
		margin-bottom: var(--hud-space-5);
	}

	.doc-title {
		font-size: var(--hud-text-xl);
		font-weight: var(--hud-weight-medium);
		color: var(--hud-text);
	}

	.doc-description {
		font-size: var(--hud-text-sm);
		color: var(--hud-text-secondary);
		margin-top: var(--hud-space-2);
	}

	.doc-meta {
		display: flex;
		align-items: center;
		gap: var(--hud-space-3);
		margin-top: var(--hud-space-3);
	}

	.badge {
		font-size: var(--hud-text-2xs);
		text-transform: uppercase;
		letter-spacing: var(--hud-tracking-wide);
		padding: 2px var(--hud-space-2);
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

	.badge.superseded,
	.badge.deprecated {
		color: var(--hud-red);
		border-color: var(--hud-red-border);
		background: var(--hud-red-soft);
	}

	.doc-date {
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
	}
</style>
