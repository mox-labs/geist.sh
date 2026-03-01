<script lang="ts">
	type PhaseStatus = 'done' | 'next' | 'planned' | 'growth';

	interface Phase {
		id: string;
		name: string;
		status: PhaseStatus;
		summary: string;
		details: string[];
	}

	interface Crate {
		name: string;
		path: string;
		role: string;
	}

	const phases: Phase[] = [
		{
			id: 'P1',
			name: 'Core Types',
			status: 'done',
			summary: 'Processor trait, Sequence compositor, PhaseResult, ProcessingMode',
			details: [
				'67 tests passing',
				'Processor trait with &self (Arc-shareable, hyper school pattern)',
				'Sequence compositor with ordered pipeline execution',
				'PhaseResult vocabulary: Continue, Mutate, Respond',
				'ProcessingMode: headers-only optimization',
				'Type-safe Metadata for inter-processor communication',
			]
		},
		{
			id: 'P1.5',
			name: 'Extension Registry',
			status: 'done',
			summary: 'Typed extension registry + access control processor + composition root',
			details: [
				'40 tests across 3 PRs (#8 registry, #9 access-control, #10 composition root)',
				'IntoProcessor factory trait with associated Config type',
				'Self-registration via inventory::submit! — zero code changes to core',
				'ProcessorRegistryBuilder → immutable ProcessorRegistry',
				'Access control processor: deny-first evaluation with rumi matchers',
				'Composition root: collect_extensions() + pipeline config',
			]
		},
		{
			id: 'P2',
			name: 'axum Adapter',
			status: 'next',
			summary: 'HTTP adapter + governed proxy binary',
			details: [
				'axum adapter translating HTTP ↔ ProcessingRequest/Response',
				'Governed proxy binary serving real traffic',
				'Integration tests with actual HTTP requests through pipeline',
			]
		},
		{
			id: 'P3',
			name: 'Composer',
			status: 'planned',
			summary: 'Intent → capability selection from catalog',
			details: [
				'Capability catalog with logical IDs',
				'Intent resolution: what the agent wants → which capability serves it',
				'Capability surface construction per session',
			]
		},
		{
			id: 'P4',
			name: 'CLI Demo',
			status: 'planned',
			summary: 'Compose → configure → edge → agent session',
			details: [
				'End-to-end CLI workflow',
				'Configure pipeline from JSON/YAML',
				'Start edge runtime + connect agent',
			]
		},
		{
			id: 'P5',
			name: 'Desktop Shell',
			status: 'planned',
			summary: 'Tauri desktop app with SvelteKit frontend',
			details: [
				'Tauri container (geist-shell)',
				'SvelteKit webview with collaborator setup',
				'Embedded geist-edge instance',
				'Tauri adapter for shell/FS/IPC scope enforcement',
			]
		},
		{
			id: 'P6+',
			name: 'Enterprise',
			status: 'growth',
			summary: 'Router compositor, OTel, pingora, xDS transport, control plane',
			details: [
				'Router compositor for multi-capability routing',
				'OpenTelemetry integration',
				'pingora adapter for high-performance deployments',
				'xDS transport for dynamic configuration',
				'Control plane: gestalt.mox.nexus managed platform',
			]
		},
	];

	const crates: Crate[] = [
		{ name: 'geist-edge', path: 'geist/edge', role: 'Composable data plane runtime — Processor trait, typed extension registry, compositors, adapters' },
		{ name: 'geist-sh', path: 'geist/bin', role: 'Binary — composition root, links extensions, starts adapter + agent runtime' },
		{ name: 'geist-acl', path: 'geist/acl', role: 'Access control processor extension' },
		{ name: 'geist-log', path: 'geist/log', role: 'Access logging processor extension' },
	];

	let expanded = $state<Set<string>>(new Set());

	function toggle(id: string) {
		const next = new Set(expanded);
		if (next.has(id)) {
			next.delete(id);
		} else {
			next.add(id);
		}
		expanded = next;
	}

	function statusLabel(s: PhaseStatus): string {
		return s === 'done' ? 'DONE' : s === 'next' ? 'NEXT' : s === 'growth' ? 'GROWTH' : 'PLANNED';
	}
</script>

<svelte:head>
	<title>gestalt | status</title>
</svelte:head>

<div class="status-page">
	<div class="terminal-header">
		<span class="prompt">$</span> geist --status
	</div>

	<section class="phase-section">
		<h2 class="section-label">Phases</h2>
		<div class="phase-table">
			{#each phases as phase}
				<button
					class="phase-row"
					class:expanded={expanded.has(phase.id)}
					onclick={() => toggle(phase.id)}
				>
					<span class="phase-id">{phase.id}</span>
					<span class="phase-name">{phase.name}</span>
					<span class="phase-summary">{phase.summary}</span>
					<span class="phase-badge {phase.status}">{statusLabel(phase.status)}</span>
					<span class="phase-chevron">{expanded.has(phase.id) ? '▾' : '▸'}</span>
				</button>
				{#if expanded.has(phase.id)}
					<div class="phase-details">
						<ul>
							{#each phase.details as detail}
								<li>{detail}</li>
							{/each}
						</ul>
					</div>
				{/if}
			{/each}
		</div>
	</section>

	<section class="crate-section">
		<h2 class="section-label">Crate Map</h2>
		<div class="crate-table">
			{#each crates as crate}
				<div class="crate-row">
					<span class="crate-name">{crate.name}</span>
					<span class="crate-path">{crate.path}</span>
					<span class="crate-role">{crate.role}</span>
				</div>
			{/each}
		</div>
	</section>
</div>

<style>
	.status-page {
		padding: var(--hud-space-8) var(--hud-space-5);
		max-width: 820px;
	}

	.terminal-header {
		font-size: var(--hud-text-lg);
		color: var(--hud-text);
		margin-bottom: var(--hud-space-8);
		padding: var(--hud-space-3) 0;
	}

	.prompt {
		color: var(--hud-accent);
		margin-right: var(--hud-space-2);
	}

	.section-label {
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
		text-transform: uppercase;
		letter-spacing: var(--hud-tracking-wider);
		margin-bottom: var(--hud-space-3);
	}

	/* --- Phase table --- */

	.phase-section {
		margin-bottom: var(--hud-space-10);
	}

	.phase-table {
		border: 1px solid var(--hud-border-subtle);
		border-radius: var(--hud-radius);
		overflow: hidden;
	}

	.phase-row {
		display: grid;
		grid-template-columns: 48px 120px 1fr auto 24px;
		gap: var(--hud-space-3);
		align-items: center;
		padding: var(--hud-space-3) var(--hud-space-4);
		border: none;
		border-bottom: 1px solid var(--hud-border-subtle);
		background: transparent;
		color: var(--hud-text);
		font-family: var(--hud-font);
		font-size: var(--hud-text-sm);
		text-align: left;
		cursor: pointer;
		width: 100%;
		transition: background var(--hud-transition);
	}

	.phase-row:last-of-type:not(.expanded) {
		border-bottom: none;
	}

	.phase-row:hover {
		background: var(--hud-bg-hover);
	}

	.phase-id {
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
		font-weight: var(--hud-weight-medium);
	}

	.phase-name {
		color: var(--hud-text);
		font-weight: var(--hud-weight-medium);
	}

	.phase-summary {
		color: var(--hud-text-muted);
		font-size: var(--hud-text-xs);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.phase-badge {
		font-size: var(--hud-text-3xs);
		text-transform: uppercase;
		letter-spacing: var(--hud-tracking-wide);
		padding: 1px var(--hud-space-2);
		border-radius: var(--hud-radius);
		border: 1px solid;
		white-space: nowrap;
	}

	.phase-badge.done {
		color: var(--hud-green);
		border-color: var(--hud-green-border);
		background: var(--hud-green-soft);
	}

	.phase-badge.next {
		color: var(--hud-amber);
		border-color: var(--hud-amber-border);
		background: var(--hud-amber-soft);
	}

	.phase-badge.planned {
		color: var(--hud-text-faint);
		border-color: var(--hud-border);
		background: transparent;
	}

	.phase-badge.growth {
		color: var(--hud-blue);
		border-color: var(--hud-blue-border);
		background: var(--hud-blue-soft);
	}

	.phase-chevron {
		color: var(--hud-text-faint);
		font-size: var(--hud-text-xs);
	}

	.phase-details {
		padding: var(--hud-space-3) var(--hud-space-4) var(--hud-space-4);
		padding-left: calc(48px + var(--hud-space-4) + var(--hud-space-3));
		border-bottom: 1px solid var(--hud-border-subtle);
		background: var(--hud-bg-surface);
	}

	.phase-details ul {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--hud-space-1);
	}

	.phase-details li {
		font-size: var(--hud-text-xs);
		color: var(--hud-text-muted);
		padding-left: var(--hud-space-3);
		position: relative;
	}

	.phase-details li::before {
		content: '·';
		position: absolute;
		left: 0;
		color: var(--hud-text-faint);
	}

	/* --- Crate table --- */

	.crate-section {
		margin-bottom: var(--hud-space-8);
	}

	.crate-table {
		border: 1px solid var(--hud-border-subtle);
		border-radius: var(--hud-radius);
		overflow: hidden;
	}

	.crate-row {
		display: grid;
		grid-template-columns: 120px 100px 1fr;
		gap: var(--hud-space-3);
		align-items: center;
		padding: var(--hud-space-3) var(--hud-space-4);
		border-bottom: 1px solid var(--hud-border-subtle);
	}

	.crate-row:last-child {
		border-bottom: none;
	}

	.crate-name {
		font-size: var(--hud-text-sm);
		color: var(--hud-accent);
		font-weight: var(--hud-weight-medium);
	}

	.crate-path {
		font-size: var(--hud-text-xs);
		color: var(--hud-text-faint);
	}

	.crate-role {
		font-size: var(--hud-text-xs);
		color: var(--hud-text-muted);
	}

	/* --- Responsive --- */

	@media (max-width: 768px) {
		.phase-row {
			grid-template-columns: 40px 1fr auto 20px;
		}

		.phase-summary {
			display: none;
		}

		.phase-details {
			padding-left: calc(40px + var(--hud-space-4) + var(--hud-space-3));
		}

		.crate-row {
			grid-template-columns: 1fr;
			gap: 2px;
		}

		.crate-path {
			display: none;
		}
	}
</style>
