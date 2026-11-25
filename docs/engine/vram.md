# Gameplay VRAM layout

Addresses below are VRAM word addresses. Sizes are physical bytes, so each `$400`-word tilemap screen block occupies 2 KiB.

The overworld uses a rearranged layout with 64x32 BG1 and BG2 tilemaps and an expanded shared character region. The BG3 reduction to a streamed 32x32 tilemap applies to dungeons as well, because both modes share the HUD and pause-menu paths; otherwise dungeons keep their vanilla layout.

## Vanilla layout

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$0FFF` | 8 KiB | BG2 64x64 tilemap |
| `$1000-$1FFF` | 8 KiB | BG1 64x64 tilemap |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$6FFF` | 8 KiB | BG3/HUD 64x64 tilemap |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## Overworld layout

The 64x32 BG1/BG2 tilemaps are at `$6000-$6FFF`, and the BG3 tilemap is at `$3C00`. OBJ and BG3 graphics keep their vanilla locations. The shared BG1/BG2 character region is 30 KiB, or 960 characters, compared with vanilla's 16 KiB and 512 characters.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$3BFF` | 30 KiB | Shared BG1/BG2 4bpp character graphics |
| `$3C00-$3FFF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$67FF` | 4 KiB | BG2 64x32 tilemap |
| `$6800-$6FFF` | 4 KiB | BG1 64x32 tilemap |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## Dungeon layout

Dungeon BG1, BG2, their graphics, and OBJ graphics keep their vanilla locations. Only the BG3 entry differs from vanilla. Dungeons keep complete 64x64 BG1 and BG2 tilemaps resident and upload whole 32x32 quadrants before scrolling between rooms; they do not stream rows or columns as the camera moves.

| VRAM range | Size | Use |
| --- | ---: | --- |
| `$0000-$0FFF` | 8 KiB | BG2 64x64 tilemap |
| `$1000-$1FFF` | 8 KiB | BG1 64x64 tilemap |
| `$2000-$3FFF` | 16 KiB | Shared BG1/BG2 4bpp character graphics |
| `$4000-$5FFF` | 16 KiB | OBJ character graphics |
| `$6000-$63FF` | 2 KiB | BG3/HUD 32x32 tilemap |
| `$6400-$6FFF` | 6 KiB | Free |
| `$7000-$7FFF` | 8 KiB | BG3/HUD character graphics |

## Register values

| Layout | `BG1SC` | `BG2SC` | `BG3SC` | `BG12NBA` |
| --- | ---: | ---: | ---: | ---: |
| Vanilla | `$13` | `$03` | `$63` | `$22` |
| Overworld | `$69` | `$61` | `$3C` | `$00` |
| Dungeons | `$13` | `$03` | `$60` | `$22` |

`BG1SC`, `BG2SC`, and `BG3SC` select each tilemap's base and dimensions. `BG12NBA` selects the shared BG1/BG2 character base. Every overworld scene uses the overworld layout.

## Randomized pickup graphics

Randomized pickups reuse the existing OBJ graphics footprints without changing the gameplay VRAM layout or borrowing Link, equipment, or follower tiles. Dynamic pickups use the item-receipt slot: characters `$24/$25` at VRAM `$4240` and `$34/$35` at `$4340`. Heart-piece carriers retain their separate fixed slot: characters `$E0/$E1` at `$4E00` and `$F0/$F1` at `$4F00`. Each footprint holds four 8x8 4bpp tiles, totaling 128 bytes. This keeps the Spectacle Rock pickup independent of the falling Ether Tablet item and of native item-receipt graphics.

The item-placement patch caches assigned heart-piece artwork in free WRAM and uploads it after the native dynamic OBJ transfers when that carrier is drawn. Native graphics loading restores the common sheet on scene changes; the next visible heart-piece draw supplies the assigned artwork again. The dynamic item buffer continues to serve ordinary previews and native receipts. Every item graphics load invalidates the preview cache, and loading a heart-piece image preserves the buffer and its cache identity. All item graphics, including rupee frames, use fixed 128-byte 4bpp ROM entries indexed by graphics ID. The patcher converts the vanilla artwork from the input ROM; custom artwork uses the same layout. `$7FF180-$7FF2FF` contains cache metadata and buffers, within the vanilla free range ending at `$7FF7FF`.

Narrow item images use transparent right-hand tiles and fit the carriers’ existing OAM allocations. Equipment previews reindex the native sword/shield artwork into common item palettes so displaying a different equipment tier does not recolor Link’s equipped sword or shield. Native receipt rendering still uses its original equipment palettes. HalfMagic uses the 16x16 icon from `data/item_icons/half-magic.png`, encoded as four 4bpp tiles in `patches/src/half_magic_graphics.inc`. Purple pixels become transparent; opaque pixels use the upper half of sprite palette 4, shared by Light World and Dark World. Both previews and receipts use this image, with 16x16 dimensions and centering. The receipt uses the normal item renderer and Half Magic's own grant and message table entries, without refilling magic. A shared loader copies the selected ROM entry into the dynamic item buffer for both pickups and receipts. The unified receipt tables reserve 256 entries each in bank `$B1`; the patcher copies their vanilla prefixes from the input ROM, while ASM supplies the Half Magic entries. The receipt bank also contains mirrors of the vanilla dungeon-mask and bottle lookup data required by the initializer. This allocation assumes that pickups sharing a footprint are never on screen together, which holds for the vanilla locations.
