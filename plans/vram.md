# Gameplay VRAM layout

Addresses below are VRAM word addresses. Sizes are physical bytes, so each `$400`-word tilemap screen block occupies 2 KiB.

Milestones 5 and 6 are global gameplay changes: their BG3 reductions apply to
both overworld and dungeons because those modes share the HUD and pause-menu
paths. Milestones 7 through 9 rearrange only overworld VRAM; dungeons retain
their vanilla BG1, BG2, graphics, and OBJ locations.

## Vanilla layout

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$0FFF` | 8 KiB | BG2 64x64 tilemap |
| `$1000-$1FFF` | 8 KiB | BG1 64x64 tilemap |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$6FFF` | 8 KiB | BG3/HUD 64x64 tilemap |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## After milestone 5

In both overworld and dungeons, BG3 becomes 32x64. Its pause-menu block moves
from `$6800` to `$6400`, releasing the unused right-hand blocks.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$0FFF` | 8 KiB | BG2 64x64 tilemap |
| `$1000-$1FFF` | 8 KiB | BG1 64x64 tilemap |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$67FF` | 4 KiB | BG3/HUD 32x64 tilemap |
| `$6800-$6FFF` | 4 KiB | Free |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## After milestone 6

In both overworld and dungeons, BG3 becomes a streamed 32x32 tilemap,
releasing another 2 KiB.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$0FFF` | 8 KiB | BG2 64x64 tilemap |
| `$1000-$1FFF` | 8 KiB | BG1 64x64 tilemap |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$63FF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$6400-$6FFF` | 6 KiB | Free |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## After milestone 7

BG2 becomes 64x32. It keeps its two horizontal screen blocks and releases its
lower two blocks.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$07FF` | 4 KiB | BG2 64x32 tilemap |
| `$0800-$0FFF` | 4 KiB | Free |
| `$1000-$1FFF` | 8 KiB | BG1 64x64 tilemap |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$63FF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$6400-$6FFF` | 6 KiB | Free |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## After milestone 8

BG1 also becomes 64x32, freeing another 4 KiB. The BG1/BG2 free ranges are
not yet contiguous.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$07FF` | 4 KiB | BG2 64x32 tilemap |
| `$0800-$0FFF` | 4 KiB | Free |
| `$1000-$17FF` | 4 KiB | BG1 64x32 tilemap |
| `$1800-$1FFF` | 4 KiB | Free |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$63FF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$6400-$6FFF` | 6 KiB | Free |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## After milestone 9

The reduced BG1/BG2 tilemaps move to `$6000-$6FFF`, and the BG3 tilemap moves
to `$3C00`. OBJ and BG3 graphics remain in place. The shared BG1/BG2 character
region expands from 16 KiB to 30 KiB, or from 512 to 960 characters.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$3BFF` | 30 KiB | Shared BG1/BG2 4bpp character graphics |
| `$3C00-$3FFF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$67FF` | 4 KiB | BG2 64x32 tilemap |
| `$6800-$6FFF` | 4 KiB | BG1 64x32 tilemap |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## Dungeon gameplay layout retained by milestone 9

Milestone 9 does not rearrange dungeon VRAM. BG1, BG2, their graphics, and OBJ
graphics retain their vanilla locations. The BG3 entry below is intentionally
non-vanilla because milestones 5 and 6 apply to dungeons as well as the
overworld. Dungeons keep complete 64x64 BG1 and BG2 tilemaps resident and
upload whole 32x32 quadrants before scrolling between rooms; they do not
stream rows or columns as the camera moves.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$0FFF` | 8 KiB | BG2 64x64 tilemap |
| `$1000-$1FFF` | 8 KiB | BG1 64x64 tilemap |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$63FF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$6400-$6FFF` | 6 KiB | Free |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## Playable-overworld register values

| Stage | `BG1SC` | `BG2SC` | `BG3SC` | `BG12NBA` |
| --- | ---: | ---: | ---: | ---: |
| Vanilla | `$13` | `$03` | `$63` | `$22` |
| Milestone 5 | `$13` | `$03` | `$62` | `$22` |
| Milestone 6 (current) | `$13` | `$03` | `$60` | `$22` |
| Milestone 7 | `$13` | `$01` | `$60` | `$22` |
| Milestone 8 | `$11` | `$01` | `$60` | `$22` |
| Milestone 9 | `$69` | `$61` | `$3C` | `$00` |

`BG1SC`, `BG2SC`, and `BG3SC` select each tilemap's base and dimensions.
`BG12NBA` selects the shared BG1/BG2 character base. Presentation modes such
as the world map may use different layouts. Dungeons retain `BG1SC=$13`,
`BG2SC=$03`, `BG3SC=$60`, and `BG12NBA=$22`.
