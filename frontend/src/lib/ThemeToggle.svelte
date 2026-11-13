<script lang="ts">
	import { tooltip } from '$lib/tooltip';
	import { onMount } from 'svelte';
	import lightWorld from '$lib/assets/light-world.png';
	import darkWorld from '$lib/assets/dark-world.png';

	let theme = $state<'light' | 'dark'>('light');

	onMount(() => {
		theme = document.documentElement.getAttribute('data-bs-theme') === 'dark' ? 'dark' : 'light';
	});

	function toggle() {
		theme = theme === 'dark' ? 'light' : 'dark';
		document.documentElement.setAttribute('data-bs-theme', theme);
		localStorage.setItem('theme', theme);
	}
</script>

<button
	class="btn btn-outline-primary btn-sm"
	onclick={toggle}
	aria-label={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
	use:tooltip={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
>
	<img class="icon-light" src={darkWorld} alt="" />
	<img class="icon-dark" src={lightWorld} alt="" />
</button>

<style>
	img {
		height: 1.5rem;
		image-rendering: pixelated;
	}

	:global(html[data-bs-theme='light']) .icon-light,
	:global(html[data-bs-theme='dark']) .icon-dark {
		display: none;
	}
</style>