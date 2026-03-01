<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import Shell from '$lib/components/shell/Shell.svelte';
	import GeistCanvas from '$lib/components/canvas/GeistCanvas.svelte';
	import Shortcuts from '$lib/components/shortcuts/Shortcuts.svelte';

	let { children } = $props();
	let isLanding = $derived($page.url.pathname === '/');
</script>

<svelte:head>
	<title>gestalt</title>
	<meta name="description" content="Policy enforcement for autonomous agents" />
</svelte:head>

<GeistCanvas />
<Shortcuts />

{#if isLanding}
	{@render children()}
{:else}
	<Shell>
		{@render children()}
	</Shell>
{/if}
