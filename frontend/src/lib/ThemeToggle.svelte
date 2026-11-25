<script lang="ts">
	import { onMount } from 'svelte';
	import linkHead from '$lib/assets/link_head.png';
	import bunnyHead from '$lib/assets/bunny_head.png';

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
	title={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
>
	<img class="icon-light" src={bunnyHead} alt="" />
	<img class="icon-dark" src={linkHead} alt="" />
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