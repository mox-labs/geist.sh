<script lang="ts">
	import { page } from '$app/stores';
	import { sections } from '$lib/nav/sections';

	let { children } = $props();
	let sidebarOpen = $state(false);

	function isActive(href: string): boolean {
		if (href === '/') return $page.url.pathname === '/';
		return $page.url.pathname === href || $page.url.pathname.startsWith(href + '/');
	}
</script>

<div class="shell">
	<nav class="sidebar" class:open={sidebarOpen}>
		<a href="/" class="sidebar-brand" onclick={() => { sidebarOpen = false; }}>
			<span class="brand-text">gestalt</span>
			<span class="brand-sub">v0.0.1</span>
		</a>

		<div class="sidebar-nav">
			{#each sections as section}
				<span class="section-label">{section.label}</span>
				{#each section.items as item}
					<a
						href={item.href}
						class="nav-item"
						class:active={isActive(item.href)}
						onclick={() => { sidebarOpen = false; }}
					>
						{item.label}
					</a>
				{/each}
			{/each}
		</div>

		<div class="sidebar-footer">
			<a href="https://mox.nexus" class="footer-link" target="_blank" rel="noopener">
				mox.nexus
			</a>
		</div>
	</nav>

	<button
		class="mobile-toggle"
		onclick={() => { sidebarOpen = !sidebarOpen; }}
		aria-label="Toggle sidebar"
	>
		{#if sidebarOpen}&times;{:else}&#9776;{/if}
	</button>

	{#if sidebarOpen}
		<button
			class="sidebar-backdrop"
			onclick={() => { sidebarOpen = false; }}
			aria-label="Close sidebar"
		></button>
	{/if}

	<main class="main-area">
		{@render children()}
	</main>
</div>

<style>
	.shell {
		display: flex;
		min-height: 100vh;
		position: relative;
		z-index: 1;
	}

	.sidebar {
		width: var(--hud-sidebar-width);
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		background: rgba(8, 12, 20, 0.92);
		border-right: 1px solid var(--hud-border-subtle);
		padding: var(--hud-sidebar-padding-y) 0;
	}

	.sidebar-brand {
		display: flex;
		align-items: baseline;
		gap: 6px;
		padding: 0 var(--hud-space-4) var(--hud-space-5);
		text-decoration: none;
		border-bottom: 1px solid var(--hud-border-subtle);
		margin-bottom: var(--hud-space-3);
	}

	.brand-text {
		font-size: var(--hud-text-base);
		font-weight: var(--hud-weight-medium);
		color: var(--hud-text);
	}

	.brand-sub {
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
	}

	.sidebar-nav {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 0 var(--hud-sidebar-padding-x);
	}

	.section-label {
		display: block;
		font-size: var(--hud-text-2xs);
		color: var(--hud-text-faint);
		text-transform: uppercase;
		letter-spacing: var(--hud-tracking-wider);
		padding: var(--hud-space-3) var(--hud-space-3) var(--hud-space-1);
	}

	.nav-item {
		display: block;
		padding: var(--hud-space-2) var(--hud-space-3);
		font-size: var(--hud-text-sm);
		color: var(--hud-text-muted);
		text-decoration: none;
		border-radius: var(--hud-radius);
		transition: all var(--hud-transition);
	}

	.nav-item:hover {
		color: var(--hud-text);
		background: var(--hud-bg-hover);
	}

	.nav-item.active {
		color: var(--hud-text);
		background: var(--hud-bg-hover);
		border-left: 2px solid var(--hud-accent);
		padding-left: calc(var(--hud-space-3) - 2px);
	}

	.sidebar-footer {
		padding: var(--hud-space-3) var(--hud-space-4) 0;
		border-top: 1px solid var(--hud-border-subtle);
	}

	.footer-link {
		font-size: var(--hud-text-xs);
		color: var(--hud-text-faint);
		text-decoration: none;
	}

	.footer-link:hover {
		color: var(--hud-text-muted);
	}

	.main-area {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		background: rgba(8, 12, 20, 0.85);
	}

	.mobile-toggle {
		display: none;
		position: fixed;
		top: var(--hud-space-2);
		left: var(--hud-space-2);
		z-index: var(--hud-z-toggle);
		background: var(--hud-bg-raised);
		border: 1px solid var(--hud-border-strong);
		color: var(--hud-text);
		font-size: 22px;
		width: 44px;
		height: 44px;
		border-radius: var(--hud-radius-md);
		cursor: pointer;
		line-height: 1;
	}

	.sidebar-backdrop {
		display: none;
	}

	@media (max-width: 1024px) {
		.sidebar { width: 180px; }
	}

	@media (max-width: 768px) {
		.sidebar {
			position: fixed;
			top: 0;
			left: 0;
			bottom: 0;
			width: 240px;
			z-index: var(--hud-z-drawer);
			transform: translateX(-100%);
			transition: transform 200ms ease-out;
			padding-top: 60px;
		}

		.sidebar.open { transform: translateX(0); }

		.nav-item {
			padding: var(--hud-space-3);
			font-size: var(--hud-text-base);
		}

		.footer-link {
			font-size: var(--hud-text-sm);
			display: inline-block;
			padding: var(--hud-space-2) 0;
		}

		.mobile-toggle {
			display: flex;
			align-items: center;
			justify-content: center;
		}

		.sidebar-backdrop {
			display: block;
			position: fixed;
			inset: 0;
			z-index: var(--hud-z-backdrop);
			background: rgba(0, 0, 0, 0.6);
			border: none;
			cursor: default;
		}

		.main-area { padding-top: 56px; }
	}
</style>
