<script lang="ts">
	import DocHeader from '$lib/components/content/DocHeader.svelte';
	import Prose from '$lib/components/content/Prose.svelte';

	let { data } = $props();
</script>

<article class="doc-page">
	<DocHeader
		title={data.decision.title}
		description={undefined}
		status={data.decision.status}
		date={data.decision.date}
	/>
	<div class="doc-content">
		<Prose md={data.decision.content} />
	</div>
</article>

<style>
	.doc-page {
		max-width: 720px;
	}

	.doc-content {
		padding: 0 var(--hud-space-5) var(--hud-space-8);
	}
</style>
