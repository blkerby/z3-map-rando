<script lang="ts">
	import LinkIcon from '$lib/assets/LinkIcon.png';
	import { resolve } from '$app/paths';
</script>

<main class="container content-width my-4">
	<div class="card hero-card shadow">
		<div class="row g-0 align-items-stretch">
			<div class="col-lg-8 card-body">
				<h1 class="display-6 fw-bold mb-3">A Link to the Past Map Rando</h1>
				<p class="mb-4 fs-5">
					The overworld and items are rearranged into a new layout every time.
				</p>
				<p class="mb-4 fs-5">
					Can you find the items needed to defeat Ganon and obtain the Triforce?
				</p>
				<a href={resolve('/generate')} class="btn btn-primary btn-lg d-block mx-auto mb-3 generate-btn">Generate Game</a>
			</div>
			<div class="col-lg-4 d-flex justify-content-center align-items-center">
				<img src={LinkIcon} alt="Sample randomized map" class="map-preview" />
			</div>
		</div>
	</div>
	<section class="info-section mt-5">
		<h2 class="h3 fw-bold mb-3">Project status</h2>
		<p>
		This randomizer is still in development. If you encounter bugs, please let us know in the <a href="https://discord.gg/Mxb5zYZeVj">Discord</a>!
		</p>
	</section>
</main>

<style>
	.hero-card {
		border: 2px solid var(--bs-border-color);
		--bs-card-border-radius: 1rem;
		border-radius: var(--bs-card-border-radius);
	}

	.map-preview {
		width: 50%;
		height: 80%;
		object-fit: contain;
		image-rendering: crisp-edges;
		image-rendering: pixelated;
	}

	.generate-btn {
	width: fit-content;
	}

	.info-section {
		color: var(--bs-body-color);
	}

	.info-section h2 {
		color: var(--bs-emphasis-color);
	}

	.info-section a {
		color: var(--bs-link-color);
	}

</style>