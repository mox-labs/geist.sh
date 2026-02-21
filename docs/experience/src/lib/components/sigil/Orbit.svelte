<script lang="ts">
	import { untrack } from 'svelte';
	import { C, spiralStart, spiralEnd, spiralTurns } from './geometry';
	import { easeOut, phase } from './math';
	import Entity from './Entity.svelte';

	interface Props {
		time: number;
		active: boolean;
		runTime: number;
	}

	let { time, active, runTime }: Props = $props();

	const stageWidth = 0.25;
	const maxProgress = 1.0;
	const orbitSpeed = 0.00003;

	let nextId = 1;
	let prevTime = 0;
	let entities: { id: number; progress: number }[] = $state([]);

	// Advance entity lifecycle each frame.
	// Only `time` and `active` are tracked — entity mutations
	// happen inside untrack to avoid circular dependency.
	$effect.pre(() => {
		const t = time;
		const isActive = active;

		untrack(() => {
			if (!isActive) {
				prevTime = t;
				return;
			}

			const dt = t - prevTime;
			prevTime = t;

			if (dt <= 0 || dt > 100) return;

			if (entities.length === 0) {
				entities = [{ id: 0, progress: 0 }];
			}

			for (let i = 0; i < entities.length; i++) {
				entities[i].progress += orbitSpeed * dt;
			}

			for (let i = entities.length - 1; i >= 0; i--) {
				if (entities[i].progress >= maxProgress) {
					entities.splice(i, 1);
				}
			}

			const youngest =
				entities.length > 0
					? entities.reduce((min, e) => (e.progress < min.progress ? e : min), entities[0])
					: null;

			if (entities.length < 4 && (!youngest || youngest.progress >= stageWidth)) {
				entities.push({ id: nextId++, progress: 0 });
			}
		});
	});

	let entityTransforms = $derived(
		entities.map((v) => {
			const p = Math.min(v.progress, maxProgress);
			const t = p / maxProgress;
			const angle = -Math.PI / 2 + t * spiralTurns * 2 * Math.PI;
			const dist = spiralStart + t * (spiralEnd - spiralStart);
			const x = C + Math.cos(angle) * dist;
			const y = C + Math.sin(angle) * dist;
			const sz = 18 + Math.min(p, 0.8) * 30;
			let op = 1;
			if (p < 0.08) op = p / 0.08;
			if (p > 0.94) op = (1.0 - p) / 0.06;
			const rot = -(runTime * 0.018) % 360;

			const p1 = easeOut(phase(p, 0, 0.2));
			const p2 = easeOut(phase(p, 0.2, 0.4));
			const p3 = easeOut(phase(p, 0.4, 0.6));
			const p4 = easeOut(phase(p, 0.6, 0.8));

			return { id: v.id, x, y, sz, op, rot, p1, p2, p3, p4 };
		})
	);
</script>

{#each entityTransforms as t (t.id)}
	<Entity
		x={t.x}
		y={t.y}
		sz={t.sz}
		op={t.op}
		rot={t.rot}
		p1={t.p1}
		p2={t.p2}
		p3={t.p3}
		p4={t.p4}
	/>
{/each}
