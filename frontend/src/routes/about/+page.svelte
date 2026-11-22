<script lang="ts">
	import version from '$build/version.txt?raw';
	import LinkIcon from '$lib/assets/LinkIcon.png';
</script>

<main class="container my-4">
	<h1 class="h2 mb-3">About</h1>
	<p>Information about this project will go here.</p>
	<img src={LinkIcon} alt="Link holding the Triforce" class="link-icon d-block mx-auto mt-4" />
	<footer class="mt-3 text-center text-body-secondary small">Version {version}</footer>
</main>

<style>
	.link-icon {
		height: 200px;
		image-rendering: crisp-edges;
		image-rendering: pixelated;
	}
</style>
