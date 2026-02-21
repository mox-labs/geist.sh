<script lang="ts">
	import { blueFlame, sparkAngles, sparkSw, sparkRaySeeds } from './geometry';

	interface Props {
		time: number;
		rayOpacities: number[]; // per-ray 0-1 (one-by-one formation)
	}

	let { time, rayOpacities }: Props = $props();

	// Per-ray flickering lengths — each varies independently
	let rayLens = $derived(
		sparkRaySeeds.map((s, i) => {
			const t1 = Math.sin(time * s.freq1 + s.phase);
			const t2 = Math.sin(time * s.freq2 + s.phase * 0.6);
			const combined = (t1 + t2 * 0.5) / 1.5;
			return s.minLen + (combined * 0.5 + 0.5) * (s.maxLen - s.minLen);
		})
	);
</script>

{#each sparkAngles as a, i}
	{@const len = rayLens[i]}
	<line
		x1={0}
		y1={0}
		x2={Math.cos(a) * len}
		y2={Math.sin(a) * len}
		stroke={blueFlame}
		stroke-width={sparkSw}
		stroke-linecap="round"
		opacity={rayOpacities[i] * 0.7}
	/>
{/each}
