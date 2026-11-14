# English text

Status: deferred. Custom messages currently use the vanilla Japanese font and renderer. Replace the font once a complete English script is ready to replace all Japanese message text.

## Current state

`messages.asm` owns the message parser hook. Message IDs from `$0200` select custom text in bank `$A0`; vanilla IDs keep their Japanese text, except the start-menu messages `$0182`/`$0183`. Custom text is written as Asar strings using the native uppercase glyphs, 14 characters per row. Choice text starts in column 4, after the arrow redrawn by vanilla messages `$0001`/`$0002`.

## Vanilla Japanese text

Messages are one stream in bank `$1C` (`$1C8000`-`$1CF356`). At boot, `CreateMessagePointers` scans it for end bytes and builds the ID-to-address table at `$7F71C0`, so message order defines the IDs.

A message byte `$00`-`$F5` selects glyph index `$000`-`$0F5`; `$F6`-`$FF` are commands, with `$FF` doubling as a space, and `$FD xx` selects kanji glyph `$100 + xx`. The renderer masks glyph indices to 9 bits, and all 512 are defined: each has a 5-byte row mask in `TextCharMasks` (`$0EB844`) and compressed rows in `TextGFX` (`$0E8000`), and the boot routine builds a 512-entry offset table at `$7F5B00`. Bank `$0E` has only `$0C` bytes free after the masks and `$222` after the renderer.

The font has uppercase A-Z (`$AA`-`$C3`), digits, `, ? !`, apostrophes (`$D8`, `$D9`), and two dots usable as a Western period (`$DB`, `$DC`); `$CD` is the Japanese `。`. The codes labeled lowercase in jpdasm (`$CF`-`$D7` and `$EC`-`$F5`) are red capitals A-Q, and `$D2`/`$D3` form a picture glyph. There is no lowercase.

The renderer positions glyphs at pixel granularity: the pen is a tile column (`$1CDD`) plus a sub-tile offset (`$1CE6`). Each glyph is drawn in a left 8-pixel part and a right part, advancing by the two entries of `RenderText_MaskerThing` (`$0EFD7C`: 8 and 3), read at `$0EF356`, `$0EF3BA`, and `$0EF495`. Each character therefore advances 11 pixels, giving 14 per row. A row is 19 tile columns (152 pixels); text overflowing a row continues on the next. With an 8-pixel advance, the existing capitals fit 19 per row and remain legible, but kana and kanji are about 11 pixels wide.

## Other fonts

The US version uses a different renderer with variable widths: 8x16 2bpp glyphs at `$0E8000` and one-byte advances indexed by character code at `$0ECADF` (read at `$0ECB8C`), e.g. 3 pixels for I, i, and l and 7 for M, m, and W.

ALTTPR patches the Japanese 1.0 ROM:

- `i18n_en.bin` replaces the message stream in place, in the vanilla format and ID order, so `CreateMessagePointers` is unchanged. It is ALTTPR's own short script, with many stubbed messages.
- English uses Japanese byte codes. `FontProperties` gives each code an optional uncompressed glyph in `newfont.bin`, a width, and flags; codes without one fall back to the Japanese glyph. Letters use a fixed 8-pixel advance with no gap, since none sets the variable-width flag. Lowercase occupies `$D0`-`$E9`, and some kana codes become punctuation, so Japanese text would be garbled.
- Hooks at the glyph fetch (`$8EF51B`), the three advance reads, and the tilemap writes implement this.
- Runtime text such as item names and hints bypasses message IDs: nine `DialogOverride` hooks redirect reads of the parsed buffer `$7F1200` to a WRAM buffer filled from tables such as `Extra_Text_Table`.

ALTTPR's capitals are the Japanese letterforms, stored uncompressed with their outline. Its lowercase, accented letters, and extra punctuation are new artwork, added in 2022 (z3randomizer commit `b392eb7`).

## Plan

Once a complete English script exists, replace all Japanese message text in one step:

1. Write the script in the vanilla message order, replacing the bank `$1C` stream, or relocate it with the boot pointer builder.
2. Assign English glyph codes, following ALTTPR's assignments if adopting its font.
3. Hook the glyph fetch and advance reads to draw uncompressed 8x16 glyphs with per-code advances. A fixed 8-pixel advance gives 19 characters per row; per-glyph widths, as in the US version, would fit more.
4. Convert `messages.asm` text to the new codes, and lengthen its rows.

Text outside messages, such as the file select screen and menus, uses separate graphics and is migrated separately.
