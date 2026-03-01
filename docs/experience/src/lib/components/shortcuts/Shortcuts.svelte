<script lang="ts">
	import { toggleInspect, isInspecting } from '$lib/components/canvas/inspect.svelte';

	let showHelp = $state(false);
	let inspecting = $derived(isInspecting());

	function handleKeydown(e: KeyboardEvent) {
		const tag = (e.target as HTMLElement)?.tagName;
		if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;

		if (e.key === 'Escape') {
			if (showHelp) {
				showHelp = false;
				e.preventDefault();
			}
			return;
		}

		if (e.key === 'i' && !e.metaKey && !e.ctrlKey && !e.altKey) {
			toggleInspect();
			e.preventDefault();
			return;
		}

		if (e.key === '?' && !e.metaKey && !e.ctrlKey && !e.altKey) {
			showHelp = !showHelp;
			e.preventDefault();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if inspecting}
	<div class="inspect-badge">[INSPECT]</div>
{/if}

{#if showHelp}
	<div class="help-overlay">
		<div class="help-header">
			<span class="help-title">Shortcuts</span>
			<button class="help-close" onclick={() => { showHelp = false; }}>Esc</button>
		</div>
		<div class="help-list">
			<div class="help-row">
				<kbd>i</kbd>
				<span>Toggle inspect mode (orbit 3D scene)</span>
			</div>
			<div class="help-row">
				<kbd>?</kbd>
				<span>Toggle this help</span>
			</div>
			<div class="help-row">
				<kbd>Esc</kbd>
				<span>Close overlays</span>
			</div>
		</div>
	</div>
{/if}

<style>
	.inspect-badge {
		position: fixed;
		top: var(--hud-space-3);
		right: var(--hud-space-3);
		z-index: 200;
		font-size: var(--hud-text-3xs);
		color: var(--hud-accent);
		background: rgba(126, 200, 227, 0.08);
		border: 1px solid var(--hud-green-border);
		padding: 2px var(--hud-space-2);
		border-radius: var(--hud-radius);
		letter-spacing: var(--hud-tracking-wide);
		pointer-events: none;
	}

	.help-overlay {
		position: fixed;
		bottom: var(--hud-space-4);
		right: var(--hud-space-4);
		z-index: 200;
		background: rgba(8, 12, 20, 0.95);
		border: 1px solid var(--hud-border-strong);
		border-radius: var(--hud-radius-md);
		padding: var(--hud-space-4);
		min-width: 260px;
	}

	.help-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--hud-space-3);
		padding-bottom: var(--hud-space-2);
		border-bottom: 1px solid var(--hud-border-subtle);
	}

	.help-title {
		font-size: var(--hud-text-xs);
		color: var(--hud-text);
		font-weight: var(--hud-weight-medium);
	}

	.help-close {
		font-size: var(--hud-text-3xs);
		color: var(--hud-text-faint);
		background: none;
		border: 1px solid var(--hud-border);
		border-radius: var(--hud-radius);
		padding: 1px var(--hud-space-2);
		cursor: pointer;
		font-family: var(--hud-font);
	}

	.help-list {
		display: flex;
		flex-direction: column;
		gap: var(--hud-space-2);
	}

	.help-row {
		display: flex;
		align-items: center;
		gap: var(--hud-space-3);
	}

	.help-row kbd {
		display: inline-block;
		min-width: 24px;
		text-align: center;
		font-size: var(--hud-text-2xs);
		font-family: var(--hud-font);
		color: var(--hud-text);
		background: var(--hud-bg-raised);
		border: 1px solid var(--hud-border);
		border-radius: var(--hud-radius);
		padding: 1px var(--hud-space-2);
	}

	.help-row span {
		font-size: var(--hud-text-xs);
		color: var(--hud-text-muted);
	}
</style>
