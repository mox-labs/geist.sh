<script lang="ts">
	import Markdown from 'svelte-exmarkdown';
	import { gfmPlugin } from 'svelte-exmarkdown/gfm';
	import type { Plugin } from 'svelte-exmarkdown';

	interface Props {
		md: string;
		plugins?: Plugin[];
	}

	let { md, plugins = [] }: Props = $props();
	const clean = $derived(md.replace(/<!--[\s\S]*?-->/g, ''));
	const allPlugins = $derived([gfmPlugin(), ...plugins]);
</script>

<div class="prose">
	<Markdown md={clean} plugins={allPlugins} />
</div>

<style>
	.prose {
		color: var(--hud-text);
		line-height: var(--hud-leading-relaxed);
		font-size: var(--hud-text-base);
	}

	.prose :global(h2) {
		font-size: var(--hud-text-lg);
		font-weight: var(--hud-weight-semibold);
		margin: var(--hud-space-8) 0 var(--hud-space-3);
		color: var(--hud-text);
	}

	.prose :global(h3) {
		font-size: var(--hud-text-base);
		font-weight: var(--hud-weight-medium);
		margin: var(--hud-space-6) 0 var(--hud-space-2);
		color: var(--hud-text);
	}

	.prose :global(p) {
		margin: var(--hud-space-4) 0;
	}

	.prose :global(ul),
	.prose :global(ol) {
		margin: var(--hud-space-4) 0;
		padding-left: var(--hud-space-6);
	}

	.prose :global(li) {
		margin: var(--hud-space-2) 0;
	}

	.prose :global(code) {
		font-family: var(--hud-font);
		font-size: 0.9em;
		background: var(--hud-bg-raised);
		padding: 0.15em 0.4em;
		border-radius: var(--hud-radius);
	}

	.prose :global(pre) {
		background: var(--hud-bg-surface);
		padding: var(--hud-space-4);
		border-radius: var(--hud-radius);
		border: 1px solid var(--hud-border-subtle);
		border-left: 3px solid var(--hud-accent);
		overflow-x: auto;
		margin: var(--hud-space-6) 0;
		font-size: var(--hud-text-xs);
		line-height: var(--hud-leading-normal);
	}

	.prose :global(pre code) {
		background: none;
		padding: 0;
	}

	.prose :global(blockquote) {
		border-left: 3px solid var(--hud-accent);
		padding-left: var(--hud-space-4);
		margin: var(--hud-space-6) 0;
		margin-right: 0;
		color: var(--hud-text-secondary);
		font-style: italic;
	}

	.prose :global(a) {
		color: var(--hud-accent);
		text-decoration: none;
		text-underline-offset: 2px;
	}

	.prose :global(a:hover) {
		text-decoration: underline;
	}

	.prose :global(hr) {
		border: none;
		border-top: 1px solid var(--hud-border-subtle);
		margin: var(--hud-space-8) 0;
	}

	.prose :global(table) {
		width: 100%;
		border-collapse: collapse;
		margin: var(--hud-space-6) 0;
		font-size: var(--hud-text-sm);
	}

	.prose :global(th),
	.prose :global(td) {
		padding: var(--hud-space-2) var(--hud-space-3);
		border: 1px solid var(--hud-border-subtle);
		text-align: left;
	}

	.prose :global(th) {
		background: var(--hud-bg-raised);
		font-weight: var(--hud-weight-medium);
		color: var(--hud-text-secondary);
	}

	.prose :global(strong) {
		font-weight: var(--hud-weight-semibold);
		color: var(--hud-text);
	}
</style>
