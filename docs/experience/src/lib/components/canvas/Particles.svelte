<script lang="ts">
	import { T } from '@threlte/core';
	import * as THREE from 'three';

	interface Props {
		time: number;
		mobile?: boolean;
	}

	let { time, mobile = false }: Props = $props();

	// ===================================================
	// GHOST + SHELL PARTICLE ECOSYSTEM
	//
	// Two populations: blue ghosts + red shell fragments.
	// When ghost passes through shell → green flash.
	// The governed moment. Intent meets structure.
	//
	// Emergence progression (simple → compound):
	//   dot → triangle → double-triangle (Star of David)
	//   → triple-triangle → hexagon → burst
	//
	// Rarer compositions = deeper pipeline traversal.
	// ===================================================

	// Blue: the ghost. Intent. Faster, more numerous.
	const BLUE_COUNT = mobile ? 25 : 50;
	// Red: the shell. Structure. Slower, more anchored.
	const RED_COUNT = mobile ? 12 : 24;
	const TOTAL = BLUE_COUNT + RED_COUNT;
	const SPREAD = 200;
	const DEPTH = 35;
	const COLLISION_DIST = 14;

	// --- Flash system (deliberately rare) ---

	const MAX_FLASHES = 14;
	const BASE_FLASH_LIFE = 500;
	const MAX_SHAPE_SEGS = 140;
	const SPAWN_CHANCE = 0.05;
	const PAIR_COOLDOWN = 2500;

	type Particle = {
		x: number;
		y: number;
		z: number;
		vx: number;
		vy: number;
		phase: number;
		freq: number;
		isBlue: boolean;
	};

	type FlashType = 'dot' | 'tri1' | 'tri2' | 'tri3' | 'hex' | 'burst';

	type Flash = {
		x: number;
		y: number;
		birth: number;
		life: number;
		type: FlashType;
		burstArms: number;
		angle: number;
	};

	// --- Particle seeds ---

	const particles: Particle[] = Array.from({ length: TOTAL }, (_, i) => {
		const isBlue = i < BLUE_COUNT;
		return {
			x: (Math.random() - 0.5) * SPREAD * 2,
			y: (Math.random() - 0.5) * SPREAD * 2,
			z: (Math.random() - 0.5) * DEPTH - 8,
			// Ghost moves faster than shell
			vx: (Math.random() - 0.5) * (isBlue ? 0.006 : 0.002),
			vy: (Math.random() - 0.5) * (isBlue ? 0.006 : 0.002),
			phase: Math.random() * Math.PI * 2,
			// Shell drifts slowly, ghost wanders more
			freq: isBlue ? 0.0003 + Math.random() * 0.0005 : 0.0001 + Math.random() * 0.0002,
			isBlue
		};
	});

	let flashes: Flash[] = [];
	const pairCooldown = new Map<number, number>();

	// --- Geometries ---

	const blueGeo = new THREE.BufferGeometry();
	const bluePos = new Float32Array(BLUE_COUNT * 3);
	blueGeo.setAttribute('position', new THREE.BufferAttribute(bluePos, 3));

	const redGeo = new THREE.BufferGeometry();
	const redPos = new Float32Array(RED_COUNT * 3);
	redGeo.setAttribute('position', new THREE.BufferAttribute(redPos, 3));

	const flashDotGeo = new THREE.BufferGeometry();
	const flashDotPos = new Float32Array(MAX_FLASHES * 3);
	flashDotGeo.setAttribute('position', new THREE.BufferAttribute(flashDotPos, 3));
	flashDotGeo.setDrawRange(0, 0);

	const flashLineGeo = new THREE.BufferGeometry();
	const flashLinePos = new Float32Array(MAX_SHAPE_SEGS * 6);
	flashLineGeo.setAttribute('position', new THREE.BufferAttribute(flashLinePos, 3));
	flashLineGeo.setDrawRange(0, 0);

	// --- Materials ---

	// Ghost — blue intent, slightly brighter than mox.nexus
	const blueMat = new THREE.PointsMaterial({
		color: '#4da6ff',
		size: 1.0,
		transparent: true,
		opacity: 0.3,
		sizeAttenuation: true,
		depthWrite: false
	});

	// Shell — red structure, dimmer, anchored presence
	const redMat = new THREE.PointsMaterial({
		color: '#d45555',
		size: 1.1,
		transparent: true,
		opacity: 0.18,
		sizeAttenuation: true,
		depthWrite: false
	});

	// Emergence — green gestalt
	const flashDotMat = new THREE.PointsMaterial({
		color: '#00FF41',
		size: 2.5,
		transparent: true,
		opacity: 0.5,
		sizeAttenuation: true,
		depthWrite: false
	});

	const flashLineMat = new THREE.LineBasicMaterial({
		color: '#00FF41',
		transparent: true,
		opacity: 0.15,
		depthWrite: false
	});

	// --- Helper: draw a triangle into the line buffer ---

	function drawTriangle(
		buf: Float32Array,
		seg: number,
		cx: number,
		cy: number,
		size: number,
		angle: number,
		z: number
	): number {
		for (let v = 0; v < 3; v++) {
			const a1 = angle + (v * Math.PI * 2) / 3;
			const a2 = angle + ((v + 1) * Math.PI * 2) / 3;
			const idx = seg * 6;
			buf[idx] = cx + Math.cos(a1) * size;
			buf[idx + 1] = cy + Math.sin(a1) * size;
			buf[idx + 2] = z;
			buf[idx + 3] = cx + Math.cos(a2) * size;
			buf[idx + 4] = cy + Math.sin(a2) * size;
			buf[idx + 5] = z;
			seg++;
		}
		return seg;
	}

	// --- Per-frame update ---

	const curPos: [number, number][] = Array.from({ length: TOTAL }, () => [0, 0]);

	$effect(() => {
		let blueIdx = 0;
		let redIdx = 0;

		particles.forEach((p, i) => {
			const drift = Math.sin(time * p.freq + p.phase) * (p.isBlue ? 0.4 : 0.15);
			const wander = Math.cos(time * p.freq * 0.6 + p.phase * 1.3) * (p.isBlue ? 0.3 : 0.1);

			let px = p.x + time * p.vx + drift;
			let py = p.y + time * p.vy + wander;

			// Wrap around edges
			px = ((((px + SPREAD) % (SPREAD * 2)) + SPREAD * 2) % (SPREAD * 2)) - SPREAD;
			py = ((((py + SPREAD) % (SPREAD * 2)) + SPREAD * 2) % (SPREAD * 2)) - SPREAD;

			curPos[i] = [px, py];

			if (p.isBlue) {
				bluePos[blueIdx * 3] = px;
				bluePos[blueIdx * 3 + 1] = py;
				bluePos[blueIdx * 3 + 2] = p.z;
				blueIdx++;
			} else {
				redPos[redIdx * 3] = px;
				redPos[redIdx * 3 + 1] = py;
				redPos[redIdx * 3 + 2] = p.z;
				redIdx++;
			}
		});

		blueGeo.attributes.position.needsUpdate = true;
		redGeo.attributes.position.needsUpdate = true;

		// --- Collision: ghost meets shell → emergence ---

		for (let bi = 0; bi < BLUE_COUNT; bi++) {
			const bp = curPos[bi];
			for (let ri = BLUE_COUNT; ri < TOTAL; ri++) {
				const rp = curPos[ri];
				const dx = bp[0] - rp[0];
				const dy = bp[1] - rp[1];
				if (dx * dx + dy * dy < COLLISION_DIST * COLLISION_DIST) {
					if (Math.random() > SPAWN_CHANCE) continue;
					if (flashes.length >= MAX_FLASHES) continue;

					const pairKey = bi * 100 + ri;
					const last = pairCooldown.get(pairKey) || 0;
					if (time - last < PAIR_COOLDOWN) continue;

					pairCooldown.set(pairKey, time);

					// Emergence progression: rarer = deeper composition
					const roll = Math.random();
					let type: FlashType;
					let life: number;
					let burstArms = 0;

					if (roll < 0.30) {
						type = 'dot';
						life = BASE_FLASH_LIFE;
					} else if (roll < 0.54) {
						type = 'tri1';
						life = BASE_FLASH_LIFE + 100;
					} else if (roll < 0.72) {
						type = 'tri2';
						life = BASE_FLASH_LIFE + 200;
					} else if (roll < 0.84) {
						type = 'tri3';
						life = BASE_FLASH_LIFE + 300;
					} else if (roll < 0.93) {
						type = 'hex';
						life = BASE_FLASH_LIFE + 250;
					} else {
						type = 'burst';
						burstArms = 3 + Math.floor(Math.random() * 4);
						life = BASE_FLASH_LIFE + 150;
					}

					flashes.push({
						x: (bp[0] + rp[0]) / 2,
						y: (bp[1] + rp[1]) / 2,
						birth: time,
						life,
						type,
						burstArms,
						angle: Math.atan2(dy, dx)
					});
				}
			}
		}

		// Clean stale cooldowns
		if (pairCooldown.size > 200) {
			for (const [key, t] of pairCooldown) {
				if (time - t > PAIR_COOLDOWN * 2) pairCooldown.delete(key);
			}
		}

		// --- Render flashes ---

		flashes = flashes.filter((f) => time - f.birth < f.life);

		let dotCount = 0;
		let segCount = 0;

		for (const f of flashes) {
			const age = (time - f.birth) / f.life;
			const z = -5 - age * 14;
			const expand = age < 0.2 ? age / 0.2 : 1;

			if (f.type === 'dot') {
				if (dotCount < MAX_FLASHES) {
					flashDotPos[dotCount * 3] = f.x;
					flashDotPos[dotCount * 3 + 1] = f.y;
					flashDotPos[dotCount * 3 + 2] = z;
					dotCount++;
				}
			} else if (f.type === 'tri1') {
				const size = 3 * expand;
				if (segCount + 3 <= MAX_SHAPE_SEGS) {
					segCount = drawTriangle(flashLinePos, segCount, f.x, f.y, size, f.angle, z);
				}
			} else if (f.type === 'tri2') {
				const size = 3.5 * expand;
				if (segCount + 6 <= MAX_SHAPE_SEGS) {
					segCount = drawTriangle(flashLinePos, segCount, f.x, f.y, size, f.angle, z);
					segCount = drawTriangle(
						flashLinePos,
						segCount,
						f.x,
						f.y,
						size,
						f.angle + Math.PI / 3,
						z
					);
				}
			} else if (f.type === 'tri3') {
				const size = 4 * expand;
				if (segCount + 9 <= MAX_SHAPE_SEGS) {
					for (let t = 0; t < 3; t++) {
						segCount = drawTriangle(
							flashLinePos,
							segCount,
							f.x,
							f.y,
							size,
							f.angle + (t * Math.PI * 2) / 9,
							z
						);
					}
				}
			} else if (f.type === 'hex') {
				const size = 4 * expand;
				if (segCount + 6 <= MAX_SHAPE_SEGS) {
					for (let v = 0; v < 6; v++) {
						const a1 = f.angle + (v * Math.PI * 2) / 6;
						const a2 = f.angle + ((v + 1) * Math.PI * 2) / 6;
						const idx = segCount * 6;
						flashLinePos[idx] = f.x + Math.cos(a1) * size;
						flashLinePos[idx + 1] = f.y + Math.sin(a1) * size;
						flashLinePos[idx + 2] = z;
						flashLinePos[idx + 3] = f.x + Math.cos(a2) * size;
						flashLinePos[idx + 4] = f.y + Math.sin(a2) * size;
						flashLinePos[idx + 5] = z;
						segCount++;
					}
				}
			} else if (f.type === 'burst') {
				const size = 3.5 * expand;
				if (segCount + f.burstArms <= MAX_SHAPE_SEGS) {
					for (let v = 0; v < f.burstArms; v++) {
						const a = f.angle + (v * Math.PI * 2) / f.burstArms;
						const idx = segCount * 6;
						flashLinePos[idx] = f.x;
						flashLinePos[idx + 1] = f.y;
						flashLinePos[idx + 2] = z;
						flashLinePos[idx + 3] = f.x + Math.cos(a) * size;
						flashLinePos[idx + 4] = f.y + Math.sin(a) * size;
						flashLinePos[idx + 5] = z;
						segCount++;
					}
				}
			}
		}

		flashDotGeo.attributes.position.needsUpdate = true;
		flashDotGeo.setDrawRange(0, dotCount);
		flashLineGeo.attributes.position.needsUpdate = true;
		flashLineGeo.setDrawRange(0, segCount * 2);
	});
</script>

<!-- Blue ghosts — intent -->
<T.Points geometry={blueGeo} material={blueMat} />

<!-- Red shell — structure -->
<T.Points geometry={redGeo} material={redMat} />

<!-- Emergence dots — green gestalt -->
<T.Points geometry={flashDotGeo} material={flashDotMat} />

<!-- Emergence structures — geometric progressions -->
<T.LineSegments geometry={flashLineGeo} material={flashLineMat} />
