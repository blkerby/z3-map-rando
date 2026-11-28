<script lang="ts">
	import version from '$build/version.txt?raw';
	import LinkIcon from '$lib/assets/LinkIcon.png';
</script>

<main class="container content-width my-4">
	<h1 class="h2 mb-3">About</h1>
	<p>Link to the Past Map Rando rearranges the overworld areas of the SNES game The Legend of Zelda: A Link to the Past. Areas are rethemed, and their edges are adapted to be able to connect in new ways. The Desert theme is complete; more themes will be coming in the future.</p>

	<p>The randomizer features fine-grained logic settings: players can select a built-in preset and adapt it by toggling individual tricks. Randomized games can be shared with others. Each player uses their own vanilla ROM to patch the game.</p>

	<h3 class="h3 mb-3">Randomizer changes</h3>
	The following are changes which the randomizer always makes (additional modifications are available as generation or customization options):
	<ul>
		<li>Swords, shields, gloves, mail, boomerangs, and Bow/Silver Arrows are progressive upgrades.</li>
		<li>Bomb and arrow capacity upgrades are placed in the item pool instead of bought at the Lake Hylia fairy.</li>
		<li>Lake Hylia fairy fills an empty bottle with a Fairy or Golden Bee for free, an unlimited number of times.</li>
		<li>Waterfall of Wishing and Pyramid fairies give a randomized upgrade, selected from among sword, shield, gloves, mail, boomerang, bow, or a bomb or arrow capacity upgrade. The player must first throw in a matching item.</li>
		<li>When having both items in a menu slot (Mushroom/Magic Powder or Shovel/Ocarina), the player can switch between them by pressing Y.</li>
		<li>The game starts in a Zelda-rescued state, so enemies spawn throughout the overworld.</li>
		<li>The rain at the start of the game lasts until you rescue Zelda, defeat a dungeon boss, open Misery Mire, or make Flute Boy disappear.</li>
		<li>The Sanctuary priest remains alive and available for healing regardless of sword level or Agahnim progress.</li>
		<li>Before defeating Agahnim, save-and-quit and outdoor/cave deaths return to the Light World even with the Mirror.</li>
		<li>In Light World, Sanctuary and Mountain Cave become spawn options after visiting their overworld area.</li>
		<li>In Dark World, Pyramid becomes a spawn option after defeating Agahnim; Link's House is an option if Mirror is unavailable.</li>
		<li>Followers are lost after a save-and-quit or outdoor/cave death.</li>
		<li>The Golden Sword deals Tempered Sword damage to Mothula.</li>
	</ul>

	<h3 class="h3 mb-3">How it works</h3>

	<p>An <a href="https://github.com/blkerby/Z3OverworldEditor">overworld editor</a> was created for this project, to allow artists to retile the game's overworld, changing overworld areas to fit new themes (currently simply the Desert theme) and adjust their edges  to allow areas to smoothly connect together. Each area has many edge variations, all of which are crafted by hand, as a set of tiling layers superimposed onto a theme's base tiling.</p>

	<p>A map generation process places areas onto an 8x8 one at a time using reinforcement learning, predicting completion outcomes (which rooms will be placed, which cells will be filled, etc.), and balance outcomes (e.g., will it lead to a common area placement or area-pair connection. On each step, the map generation model scores a set of candidate placements, selecting higher-scoring candidates with higher probability, while respecting constraints imposed by the geometry and supported edge variant sets. After a large enough set of maps are generated, a weighted sampling process is used to form a final pool of 100,000 maps, with the weights being optimized to result in a well-balanced result. An individual map in the pool determines the placement positions of the 40 overworld areas. During game generation, a map from the pool is selected, and edge variations are applied to it: e.g. for a given pair of adjacent areas, they may connect with a passage, of various possible shapes/positions, or be closed off from each other.</p>

	<p>Once a map is fully determined, keys are placed: the randomizer associates each small key with a specific locked door; this is intended to avoid the possibility of the player "wasting" a key by using it at an unintended location. However, the in-game behavior for this is not yet implemented, so currently keys are kept in their vanilla locations.</p>

	<p>After keys are assigned, the randomizer places remaining items in a sequence of waves: it determines the set of locations which are logically reachable from the start of the game, then randomly selects items from the pool of items to fill them. It checks whether the selected items provide progression to reach at least one new item location. If it does, it is accepted; otherwise a new item selection is drawn at random from the pool. On a given wave, there is a limit to the number of redraws that the randomizer will attempt; once this limit is reached, the randomizer discards the whole attempt and starts fresh with a new map. Item placement only succeeds when all item locations become logically reachable.</p>

	<h3 class="h3 mb-3">People and credits</h3>

	<p>The following people have directly worked on this randomizer, its underlying logic, and/or retheming:</p>
	<ul>
	<li>Game design: kyleb, Maddo</li>
	<li>Map generation, overworld editor, randomizer back-end, and CLI: Maddo</li>
	<li>Randomizer front-end: alch3mi57, Maddo</li>
	<li>Logic: kyleb, Maddo, Minnie Trethewey</li>
	<li>Overworld retheming: kyleb, Duck, CyberMantis</li>
	</ul>

	<p>The randomizer draws on ideas and patches from other randomizers. It is made possible by a community of people who have built up knowledge of the game over many years.</p>
	
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
