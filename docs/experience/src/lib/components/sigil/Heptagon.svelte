<script lang="ts">
	import { red, heptOutline, heptPerimeter, heptVerts, heptN } from './geometry';
	import { easeOut, t01 } from './math';

	interface Props {
		drawProgress: number; // 0-1
	}

	let { drawProgress }: Props = $props();
</script>

<!-- Sharp outline (no glow — glow is star-only) -->
<path
	d={heptOutline}
	fill="none"
	stroke={red}
	stroke-width={3.5}
	stroke-linejoin="round"
	opacity={drawProgress > 0 ? 0.9 : 0}
	stroke-dasharray={heptPerimeter}
	stroke-dashoffset={heptPerimeter * (1 - drawProgress)}
/>

<!-- Vertex dots — appear as outline reaches each vertex -->
{#each heptVerts as v, i}
	{@const threshold = (i + 0.5) / heptN}
	{@const dotOp =
		drawProgress >= threshold
			? easeOut(t01(drawProgress, threshold, Math.min(1, threshold + 0.12)))
			: 0}
	<circle cx={v.x} cy={v.y} r={2.5} fill={red} opacity={0.5 * dotOp} />
{/each}
