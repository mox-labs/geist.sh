<script lang="ts">
	import { Canvas } from '@threlte/core';
	import GeistScene from './GeistScene.svelte';
	import { browser } from '$app/environment';
	import { isInspecting } from './inspect.svelte';

	let inspecting = $derived(isInspecting());
</script>

{#if browser}
	<div class="canvas-wrapper" class:inspecting>
		<Canvas
			colorManagementEnabled={false}
			renderMode="always"
			toneMapping={0}
		>
			<GeistScene />
		</Canvas>
	</div>
{/if}

<style>
	.canvas-wrapper {
		position: fixed;
		inset: 0;
		z-index: 0;
		background: #080c14;
		pointer-events: none;
	}

	.canvas-wrapper.inspecting {
		pointer-events: auto;
		cursor: grab;
	}

	.canvas-wrapper :global(canvas) {
		display: block;
		width: 100% !important;
		height: 100% !important;
	}
</style>
