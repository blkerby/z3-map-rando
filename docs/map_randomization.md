# Map randomization

The `map_layout` setting selects `"vanilla"` or `"random"`; the bundled default is `"random"`. A random layout rearranges the overworld rooms on the vanilla 8x8 grid, using a layout from a precomputed pool (see [the rearranger](architecture.md#rearranger)), and chooses edge variants for the borders between rooms. Each Light World room and its Dark World counterpart are one placement unit, sharing their position and edge variants; specialized variants describe world-specific behavior. Each overworld room carries its theme in the layout, and special areas take the theme of their parent room. Only the Desert theme is supported, and generation never substitutes another theme or falls back to the vanilla layout. No Python runs during generation or patching.

## Map pool

The pool is a `Z3MAPPL1` file produced by z3-map-gen; its format is specified in that project's `docs/map_pool.md`. The randomizer's pool at `maps/map_pool.bin` holds 100,000 layouts of 40 source rooms, each record giving every room's top-left cell as `y * 8 + x`. The header lists the source room IDs, which are resolved through the logic catalog rather than assumed to match catalog indices, and the SHA-256 of the room catalog the pool was built from.

The pool guarantees a complete, compatible packing of the rooms, but it does not choose edge variants or prove reachability under the randomizer's logic; generation does both. The `map_pool` entry in `catalog-build.toml` configures the pool path, the CLI's `--map-pool` overrides it, and bundled builds embed the configured pool. Vanilla generation and patching never load the pool.

## Layout selection

For a selected pool record, `generator::map` fills the 8x8 grid with the rooms' footprints and enumerates each pair of touching perimeter segments once, including each segment separately where two large rooms share a side. For each pair it intersects the variants both segments support, excludes `closed` when an open choice exists, and chooses uniformly among the rest; both sides and both worlds use that choice. Segments on the board's edge are closed. There are no weights, connectivity heuristics, or backtracking.

The vanilla layout keeps vanilla positions and the vanilla connections from the source `transitions`, and uses each area's `Main` layer directly; it does not use edge variants.

Positions and variants come from the caller's seeded RNG in a deterministic order. The seed saves each room's position and theme, whether the layout is vanilla, and the open borders with their variants (see [seed data](data-formats/seed-data.md)), so patching needs neither the pool nor the RNG.

## Traversal graph

Room-local strats, item locations, interior connections, and event identities are independent of placement. For a random layout, the selected variants' endpoints replace the vanilla screen-boundary connections; moving a room moves its entrances, teleports, whirlpools, and flute spot along with it without changing their logical destinations. The graph is otherwise built as usual, including follower handling, obstacle resets, save-and-quit, and event states (see [logic](generation/logic.md)).

| Variant | Connections |
| --- | --- |
| Ordinary land or water openings | Ordered opening references paired within each world |
| `closed` | None |
| `light-only` | The ordinary connection in the Light World only |
| `light-and-mirror` | The ordinary Light World connection, plus the directed Dark-to-Light Mirror route where its endpoints exist |
| An opening marked `"return"` | Entry and return through that opening only, without reaching the neighboring room's other nodes |

The source semantics of each variant are documented in z3-json-data's [`edge-variants.md`](../z3-json-data/docs/edge-variants.md).

## Reachability gate and retries

Before Small Key assignment, a candidate layout must let every item location be discovered with full inventory: the complete configured item pool, fixed items, prizes, every Big Key, and access through every Small Key door, under the player's logic settings. Discovery requires the location's collection action to execute, starting from empty discovery state and no placements, so assumed inventory never pre-discovers a location. Unreachable vertices do not matter as long as every item location is discovered. This gate is optimistic; progression-safe placement still follows.

Each complete generation attempt selects a pool layout and tries up to 10 edge-variant selections on it until one passes the gate. If none passes, or if Small Key assignment or item placement later fails, the attempt moves on to a new layout; each attempt is charged once, whichever stage fails, within the overall budget of 1,000 attempts (see [item placement](generation/placement.md#retry-budgets)). Temporary full-inventory state is discarded, and real key assignment and placement reuse the accepted graph. Generation also checks the layout against the retiling catalog's palette profiles before placing items, and a layout whose palette search fails consumes an attempt. Rejections are logged with their reason; invalid input or catalog errors propagate rather than being retried.

## ROM topology

The engine keeps logical area identities in `$8A` and `$040A`, with their vanilla meanings and lifetimes, and resolves physical placement separately ([`overworld_topology.asm`](../patches/src/overworld_topology.asm)). Most consumers of these addresses select logical behavior, such as graphics, palettes, music, persistent state, secrets, entrances, and events; only consumers that use them as geometry, such as sprite-coordinate origins, screen-size and camera setup, and the BG streamer's window origin, look up the physical position instead. `$0700` remains the coordinate-derived cell under Link and is not an area anchor.

The patcher writes two 64-byte tables from the saved layout:

| Table | Index | Value |
| --- | --- | --- |
| `SlotToArea` | Physical cell | Logical Light World area owning that cell |
| `AreaToAnchor` | Logical Light World area | Physical top-left cell |

Both worlds share placement, so the tables omit the world bit `$40`, which is stripped before lookup and reapplied afterward. Every cell of a large area maps to the same owner. Physical placement is fixed for a seed, so no additional RAM state caches it.

Special overworld areas stay outside the shuffled grid, keeping their IDs, internal layouts, coordinate systems, and camera setup; the engine distinguishes them before applying the normal lookup. Their entry triggers move with their parent area. Returns restore the parent's cached identity, coordinates, and camera, which already describe the relocated area, while table-driven destinations in the normal overworld are relocated explicitly.

## Assets and gameplay data

A random layout composes each area's ALTTPRetiling layers whose conditions match the saved edge variants, compiling both artwork and collision from them; vanilla uses `Main` alone. Palette coexistence, character allocation, and transition asset schedules follow the layout's actual adjacencies (`WorldLayout::build_scrollable_transition_pairs`), and allocation failures are reported rather than dropping artwork.

Per-area data moves with its logical owner, keeping area-relative coordinates: maps, BG1 backgrounds, animation tracks, entrance lists and exterior returns, special transitions, cutscenes, sprites, pickups and secrets, dynamic tiles, Mirror and portal landings, flute and whirlpool destinations, and spawn and save destinations. Persistence stays attached to the logical area rather than to whichever area occupies a vanilla slot. Flute locations keep their authored numbers.

## World map

The Light and Dark World Mode 7 maps are generated from the final layout ([`overworld_world_map.asm`](../patches/src/overworld_world_map.asm)), using each area's authored map grid with the same layers as its playable tiling. The `Main` grids of `Light World Clouds` and `Dark World Clouds` form each map's background, with the areas overlaid in the central region; unpainted cloud tiles stay blank. Markers for Link, dungeons, portals, and flute spots follow the layout.

## Scope

Additional themes, edge-variant weights, connection search or backtracking, independent world layouts, entrance shuffling, geographic flute numbering, and object-placement overrides are not supported.
