<script lang="ts">
	import { green, treyaR, treyaArmEnd, dArcPath } from './geometry';

	interface Props {
		ox: number;
		oy: number;
		rot: number;
		opacity: number;
	}

	let { ox, oy, rot, opacity }: Props = $props();
</script>

<g transform="translate({ox}, {oy}) rotate({rot})" {opacity}>
	<path d={dArcPath} fill="none" stroke={green} stroke-width={3} stroke-linecap="round" />
	<line
		x1={treyaR}
		y1={0}
		x2={treyaArmEnd}
		y2={0}
		stroke={green}
		stroke-width={3}
		stroke-linecap="round"
	/>
</g>
