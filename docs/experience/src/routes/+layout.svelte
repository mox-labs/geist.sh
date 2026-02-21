<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import Shell from '$lib/components/shell/Shell.svelte';

	let { children } = $props();
	let isLanding = $derived($page.url.pathname === '/');
</script>

<svelte:head>
	<title>gestalt</title>
	<meta name="description" content="Policy enforcement for autonomous agents" />
</svelte:head>

{#if isLanding}
	{@render children()}
{:else}
	<Shell>
		{@render children()}
	</Shell>
{/if}
