<script lang="ts">
	import '../styles/theme.scss';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import favicon from '$lib/assets/MapScrollFavicon.svg';
	import DiscordIcon from '$lib/assets/DiscordIcon.png';
	import ThemeToggle from '$lib/ThemeToggle.svelte';
	import RomDialog from '$lib/rom/RomDialog.svelte';
	import RomMenu from '$lib/rom/RomMenu.svelte';
	import { provideRomStore } from '$lib/rom/store.svelte';
	import 'bootstrap-icons/font/bootstrap-icons.css';

	let { children } = $props();

	const rom = provideRomStore();

	const version = 'v1'; // bump this yourself as you release updates

	const navLinks: { href: '/' | '/generate' | '/releases' | '/logic' | '/about'; label: string }[] = [
		{ href: '/', label: 'Home' },
		{ href: '/generate', label: 'Generate' },
		{ href: '/releases', label: 'Releases' },
		{ href: '/logic', label: 'Logic' },
		{ href: '/about', label: 'About' }
	];

	onMount(async () => {
		void rom.init();
		await import('bootstrap/dist/js/bootstrap.bundle.min.js');
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<nav class="navbar navbar-world navbar-expand-lg px-3 py-1" data-bs-theme="dark">
	<div class="container-fluid">
		<a class="navbar-brand d-flex align-items-center gap-2" href={resolve('/')}>
			<img src={DiscordIcon} alt="" class="brand-icon" />
		</a>

		<button
			class="navbar-toggler"
			type="button"
			data-bs-toggle="collapse"
			data-bs-target="#mainNav"
			aria-controls="mainNav"
			aria-expanded="false"
			aria-label="Toggle navigation"
		>
			<span class="navbar-toggler-icon"></span>
		</button>

		<div class="collapse navbar-collapse" id="mainNav">
			<ul class="navbar-nav me-auto mb-2 mb-lg-0">
				{#each navLinks as link (link.href)}
					<li class="nav-item">
						<a
							class="nav-link"
							class:active={page.url.pathname === link.href}
							aria-current={page.url.pathname === link.href ? 'page' : undefined}
							href={resolve(link.href)}
						>
							{link.label}
						</a>
					</li>
				{/each}
			</ul>

			<div class="d-flex align-items-center gap-3">
				<a class="navbar-icon-link d-flex align-items-center gap-1" href="https://github.com/" target="_blank" rel="noreferrer">
					<i class="bi bi-github"></i> GitHub
				</a>
				<a class="navbar-icon-link d-flex align-items-center gap-1" href="https://discord.gg/Mxb5zYZeVj" target="_blank" rel="noreferrer">
					<i class="bi bi-discord"></i> Discord
				</a>
				<RomMenu {rom} />
				<ThemeToggle />
				<span class="version-badge">{version}</span>
			</div>
		</div>
	</div>
</nav>

{@render children()}

<RomDialog {rom} />

<style>
	.brand-icon {
		height: 3rem;
		width: 3rem;
		image-rendering: crisp-edges;
		image-rendering: pixelated;
		border-radius: 0.5rem;
	}

	.navbar-icon-link {
		color: inherit;
		text-decoration: none;
		font-size: 0.95rem;
	}

	.navbar-icon-link:hover {
		text-decoration: underline;
	}

	.version-badge {
		font-size: 0.85rem;
		opacity: 0.75;
	}

	.nav-link.active {
		font-weight: 600;
		color: var(--bs-emphasis-color) !important;
		border-bottom: 2px solid var(--bs-emphasis-color);
	}

	.navbar-world {
		--bs-navbar-color: #ffffff;
		--bs-navbar-hover-color: #ffffff;
		--bs-navbar-active-color: #ffffff;
		--bs-navbar-brand-color: #ffffff;
		--bs-navbar-brand-hover-color: #ffffff;
	}

	.navbar-icon-link {
		color: #ffffff !important;
	}

	:global(main.container) {
		max-width: 1120px;
	}

</style>