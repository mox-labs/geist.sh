<script lang="ts">
	import DocHeader from '$lib/components/content/DocHeader.svelte';
	import Prose from '$lib/components/content/Prose.svelte';

	let { data } = $props();

	const sectionLabel = $derived(
		data.doc.section === 'overview' ? 'overview'
		: data.doc.section === 'architecture' ? 'concepts'
		: 'reference'
	);
</script>

<svelte:head>
	<title>gestalt | {data.doc.title}</title>
</svelte:head>

<article class="doc-page">
	<nav class="breadcrumb" aria-label="Breadcrumb">
		<a href="/docs">docs</a>
		<span class="sep">/</span>
		<span>{sectionLabel}</span>
		<span class="sep">/</span>
		<span class="current">{data.doc.slug}</span>
	</nav>
	<DocHeader
		title={data.doc.title}
		description={data.doc.description}
	/>
	<div class="doc-content">
		<Prose md={data.doc.content} />
	</div>
</article>

<style>
	.doc-page {
		max-width: 720px;
	}

	.breadcrumb {
		padding: var(--hud-space-4) var(--hud-space-5) 0;
		font-size: var(--hud-text-xs);
		color: var(--hud-text-faint);
		display: flex;
		align-items: center;
		gap: var(--hud-space-1);
	}

	.breadcrumb a {
		color: var(--hud-text-muted);
		text-decoration: none;
	}

	.breadcrumb a:hover {
		color: var(--hud-text);
	}

	.sep {
		color: var(--hud-text-faint);
	}

	.current {
		color: var(--hud-text-secondary);
	}

	.doc-content {
		padding: 0 var(--hud-space-5) var(--hud-space-8);
	}
</style>
