<script lang="ts">
	import DocHeader from '$lib/components/content/DocHeader.svelte';
	import Prose from '$lib/components/content/Prose.svelte';

	let { data } = $props();
</script>

<article class="doc-page">
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

	.doc-content {
		padding: 0 var(--hud-space-5) var(--hud-space-8);
	}
</style>
