<script lang="ts">
	import type { Snippet } from 'svelte';
	import { V, C, green, sparkRayCount } from './geometry';
	import { t01, easeOut } from './math';

	import SparkDot from './SparkDot.svelte';
	import SparkRays from './SparkRays.svelte';
	import Orbit from './Orbit.svelte';

	interface Props {
		size?: number;
		center?: Snippet<[{ time: number }]>;
	}

	let { size = 400, center }: Props = $props();

	// ═══════════════════════════════════════════════════
	// CINEMATIC BOOT TIMELINE (ms)
	// Shared skeleton — center fills its own sub-phases.
	// ═══════════════════════════════════════════════════

	const B_DOT = [600, 1300] as const;
	const B_RAYS = [1500, 4000] as const;
	const B_PULSE = [10500, 11000] as const;
	const B_END = 11300;

	const rayStagger = (B_RAYS[1] - B_RAYS[0]) / sparkRayCount;

	// ═══ STATE ═══
	let time = $state(0);

	// ═══ ANIMATION LOOP ═══

	$effect(() => {
		let rafId: number;
		let last = performance.now();
		let motionChecked = false;

		function tick(now: number) {
			const dt = now - last;
			last = now;

			if (!motionChecked) {
				motionChecked = true;
				if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
					time = B_END;
				}
			}

			time += dt;
			rafId = requestAnimationFrame(tick);
		}

		rafId = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(rafId);
	});

	// ═══ DERIVED ═══

	let booting = $derived(time < B_END);
	let runTime = $derived(Math.max(0, time - B_END));

	let bDot = $derived(easeOut(t01(time, B_DOT[0], B_DOT[1])));

	let bRayOpacities = $derived(
		Array.from({ length: sparkRayCount }, (_, i) => {
			const start = B_RAYS[0] + i * rayStagger;
			return easeOut(t01(time, start, start + 400));
		})
	);

	let bPulseGlow = $derived((() => {
		const p = t01(time, B_PULSE[0], B_PULSE[1]);
		return p > 0 && p < 1 ? Math.sin(p * Math.PI) : 0;
	})());

	let heptRot = $derived(booting ? 0 : -(runTime * 0.008) % 360);
</script>

<svg
	viewBox="0 0 {V} {V}"
	width={size}
	height={size}
	class="sigil"
	role="img"
	aria-label="sigil — assembly and emergence"
>
	<defs>
		<filter id="glow-star" x="-30%" y="-30%" width="160%" height="160%">
			<feGaussianBlur stdDeviation="2" />
		</filter>
	</defs>

	<!-- ═══ ORBITAL ENTITIES (behind center — after boot) ═══ -->

	<Orbit {time} active={!booting} {runTime} />

	<!-- ═══ ASSEMBLY PULSE ═══ -->

	{#if bPulseGlow > 0}
		<circle cx={C} cy={C} r={72} fill="none" stroke={green} stroke-width={1.5} opacity={bPulseGlow * 0.4} />
		<circle cx={C} cy={C} r={50} fill={green} opacity={bPulseGlow * 0.06} />
	{/if}

	<!-- ═══ CENTER (rotating group — project-specific via snippet) ═══ -->

	<g transform="translate({C}, {C}) rotate({heptRot})">
		{#if center}
			{@render center({ time })}
		{/if}
	</g>

	<!-- ═══ SPARK (on top, fixed — never rotates) ═══ -->

	<g transform="translate({C}, {C})">
		<SparkRays {time} rayOpacities={bRayOpacities} />
		<SparkDot opacity={bDot} />
	</g>
</svg>

<style>
	.sigil {
		max-width: 100%;
		height: auto;
		filter: drop-shadow(0 0 6px rgba(0, 255, 65, 0.35))
			drop-shadow(0 0 20px rgba(0, 255, 65, 0.12))
			drop-shadow(0 0 8px rgba(77, 166, 255, 0.2));
	}
</style>
