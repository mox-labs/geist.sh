<script lang="ts">
	import { T, useTask, useThrelte } from '@threlte/core';
	import { OrbitControls } from '@threlte/extras';
	import * as THREE from 'three';
	import { EffectComposer, EffectPass, RenderPass, BloomEffect } from 'postprocessing';
	import Particles from './Particles.svelte';
	import { isInspecting } from './inspect.svelte';
	import { hasVisited, markVisited } from './visited.svelte';

	// --- Animation clock ---

	const B_END = 12600;
	const skipBoot = hasVisited();

	let time = $state(skipBoot ? B_END : 0);
	let motionChecked = false;
	let bootMarked = false;

	useTask((delta) => {
		if (!motionChecked) {
			motionChecked = true;
			if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
				time = B_END;
				return;
			}
		}

		time += delta * 1000;

		if (!bootMarked && time >= B_END) {
			bootMarked = true;
			markVisited();
		}
	});

	// --- Responsive camera ---

	let cameraZ = $state(400);

	// --- Mobile performance guard ---

	const isMobile = typeof window !== 'undefined' && window.innerWidth < 768;

	// --- Renderer + scene setup ---

	const { renderer, scene, camera, autoRender, renderStage } = useThrelte();

	scene.background = new THREE.Color(0x080c14);
	renderer.toneMapping = THREE.NoToneMapping;
	renderer.outputColorSpace = THREE.SRGBColorSpace;

	// --- Post-processing (bloom — disabled on mobile) ---

	autoRender.set(!isMobile);

	let composer: EffectComposer | null = null;

	function setupComposer() {
		if (isMobile) return;
		const cam = camera.current;
		if (!cam) return;

		composer = new EffectComposer(renderer, {
			frameBufferType: THREE.HalfFloatType
		});

		composer.addPass(new RenderPass(scene, cam));

		const bloom = new BloomEffect({
			luminanceThreshold: 0.3,
			luminanceSmoothing: 0.08,
			mipmapBlur: true,
			intensity: 2.8,
			radius: 0.75
		});

		composer.addPass(new EffectPass(cam, bloom));
		composer.setSize(window.innerWidth, window.innerHeight);
	}

	$effect(() => {
		if (!isMobile && camera.current) {
			setupComposer();
		}
	});

	useTask(
		() => {
			if (composer) {
				composer.render();
			}
		},
		{ stage: renderStage, autoInvalidate: false }
	);

	// --- Responsive camera + composer resize ---

	$effect(() => {
		const handleResize = () => {
			const aspect = window.innerWidth / window.innerHeight;
			cameraZ = Math.max(400, 110 / (Math.tan((18 * Math.PI) / 180) * aspect));
			composer?.setSize(window.innerWidth, window.innerHeight);
		};
		handleResize();
		window.addEventListener('resize', handleResize);
		return () => window.removeEventListener('resize', handleResize);
	});

	let inspecting = $derived(isInspecting());
</script>

<T.PerspectiveCamera makeDefault position={[0, 0, cameraZ]} fov={36} />

{#if inspecting}
	<OrbitControls
		enableDamping
		dampingFactor={0.08}
		enableZoom
		enablePan={false}
		minDistance={100}
		maxDistance={800}
	/>
{/if}

<Particles {time} mobile={isMobile} />
