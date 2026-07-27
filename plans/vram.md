# Overworld VRAM layout

Addresses below are VRAM word addresses. Sizes are physical bytes, so each `$400`-word tilemap screen block occupies 2 KiB.

## Current layout

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$0FFF` | 8 KiB | BG2 64x64 tilemap |
| `$1000-$1FFF` | 8 KiB | BG1 64x64 tilemap |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$6FFF` | 8 KiB | BG3/HUD 64x64 tilemap |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## After milestone 5

BG3 becomes 32x64. Its pause-menu block moves from `$6800` to `$6400`,
releasing the unused right-hand blocks.

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

BG3 becomes a streamed 32x32 tilemap, releasing another 2 KiB.

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

BG1 becomes 64x32. It keeps its two horizontal screen blocks and releases its
lower two blocks.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$0FFF` | 8 KiB | BG2 64x64 tilemap |
| `$1000-$17FF` | 4 KiB | BG1 64x32 tilemap |
| `$1800-$1FFF` | 4 KiB | Free |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$63FF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$6400-$6FFF` | 6 KiB | Free |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## After milestone 8

BG2 also becomes 64x32, freeing another 4 KiB. The BG1/BG2 free ranges are
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

BG1 moves to `$0800`, beside BG2. The shared BG1/BG2 character base moves from `$2000` to `$1000`, consuming the contiguous 8 KiB released by the smaller tilemaps. This expands the 4bpp character region from 16 KiB to 24 KiB, or from 512 to 768 characters.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$07FF` | 4 KiB | BG2 64x32 tilemap |
| `$0800-$0FFF` | 4 KiB | BG1 64x32 tilemap |
| `$1000-$3FFF` | 24 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$63FF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$6400-$6FFF` | 6 KiB | Free |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## Playable-overworld register values

| Stage | `BG1SC` | `BG2SC` | `BG3SC` | `BG12NBA` |
| --- | ---: | ---: | ---: | ---: |
| Current | `$13` | `$03` | `$63` | `$22` |
| Milestone 5 | `$13` | `$03` | `$62` | `$22` |
| Milestone 6 | `$13` | `$03` | `$60` | `$22` |
| Milestone 7 | `$11` | `$03` | `$60` | `$22` |
| Milestone 8 | `$11` | `$01` | `$60` | `$22` |
| Milestone 9 | `$09` | `$01` | `$60` | `$11` |

`BG1SC`, `BG2SC`, and `BG3SC` select each tilemap's base and dimensions.
`BG12NBA` selects the shared BG1/BG2 character base. Presentation modes such
as the world map may use different layouts.
