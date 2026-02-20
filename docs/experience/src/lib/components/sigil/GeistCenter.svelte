<script lang="ts">
	import { armDefs } from './geometry';
	import { t01, easeOut, easeOutBack, lerp } from './math';

	import TreyaArm from './TreyaArm.svelte';
	import Star from './Star.svelte';
	import Heptagon from './Heptagon.svelte';

	interface Props {
		time: number;
	}

	let { time }: Props = $props();

	// ═══════════════════════════════════════════════════
	// GEIST.SH CENTER — boot sub-phases (ms)
	// Fits within the shared Sigil timeline window.
	// ═══════════════════════════════════════════════════

	const B_STAR = [4300, 5800] as const;
	const B_HEPT = [5500, 7000] as const;
	const B_ARMS_SHOW = 7300;
	const B_ARM_SHOW_DUR = 400;
	const B_ARM_SHOW_LAG = 300;
	const B_ARMS_MOVE = [8500, 10500] as const;
	const B_ARM_MOVE_LAG = 200;

	let bStar = $derived(easeOut(t01(time, B_STAR[0], B_STAR[1])));
	let bHept = $derived(easeOut(t01(time, B_HEPT[0], B_HEPT[1])));

	let armTransforms = $derived(
		armDefs.map((arm, i) => {
			const showStart = B_ARMS_SHOW + i * B_ARM_SHOW_LAG;
			const appear = easeOut(t01(time, showStart, showStart + B_ARM_SHOW_DUR));

			const moveStart = B_ARMS_MOVE[0] + i * B_ARM_MOVE_LAG;
			const moveDur = B_ARMS_MOVE[1] - B_ARMS_MOVE[0];
			const move = easeOutBack(t01(time, moveStart, moveStart + moveDur));

			const ox = lerp(arm.startX, 0, move);
			const oy = lerp(arm.startY, 0, move);
			const rot = lerp(arm.startDeg, arm.finalDeg, move);
			const opacity = appear;

			return { ox, oy, rot, opacity };
		})
	);
</script>

<!-- Layer 1: Triskelion arms (deepest — behind red) -->
{#each armTransforms as arm}
	<TreyaArm ox={arm.ox} oy={arm.oy} rot={arm.rot} opacity={arm.opacity} />
{/each}

<!-- Layer 2: Star (glow + sharp) -->
<Star drawProgress={bStar} />

<!-- Layer 3: Heptagon (sharp only) -->
<Heptagon drawProgress={bHept} />
