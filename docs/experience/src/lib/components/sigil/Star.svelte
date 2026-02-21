<script lang="ts">
	import { red, heptStar, starPerimeter } from './geometry';

	interface Props {
		drawProgress: number; // 0-1
	}

	let { drawProgress }: Props = $props();
</script>

<!-- Glow layer (wider, dimmer, blurred) -->
<path
	d={heptStar}
	fill="none"
	stroke={red}
	stroke-width={4}
	stroke-linejoin="round"
	opacity={drawProgress > 0 ? 0.12 : 0}
	stroke-dasharray={starPerimeter}
	stroke-dashoffset={starPerimeter * (1 - drawProgress)}
	filter="url(#glow-star)"
/>

<!-- Sharp line -->
<path
	d={heptStar}
	fill="none"
	stroke={red}
	stroke-width={1.5}
	stroke-linejoin="round"
	opacity={drawProgress > 0 ? 0.35 : 0}
	stroke-dasharray={starPerimeter}
	stroke-dashoffset={starPerimeter * (1 - drawProgress)}
/>
