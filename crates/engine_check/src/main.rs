use anyhow::{Context, Result};
use clap::Parser;
use patcher::{
    Patcher, PcAddr, SnesAddr,
    import::{Importer, Tile16},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const VANILLA_ROM_SHA256: &str = "794e040b02c7591b59ad8843b51e7c619b88f87cddc6083a8e7a4027b96a2271";
const FLAT_MAPS_START: SnesAddr = SnesAddr(0x200000);
const FLAT_MAP_POINTERS_START: SnesAddr = SnesAddr(0x27e000);
const MAP16_DEFINITION_STARTS: [SnesAddr; 4] = [
    SnesAddr(0x288000),
    SnesAddr(0x298000),
    SnesAddr(0x2a8000),
    SnesAddr(0x2b8000),
];
const MAP16_PROPERTY_STARTS: [SnesAddr; 4] = [
    SnesAddr(0x2c8000),
    SnesAddr(0x2cc000),
    SnesAddr(0x2d8000),
    SnesAddr(0x2dc000),
];

#[derive(Parser)]
struct Args {
    input_rom: PathBuf,
    output_rom: PathBuf,
}

fn split_map16_definitions(tiles16: &[Tile16]) -> [Vec<u8>; 4] {
    std::array::from_fn(|quadrant| {
        tiles16
            .iter()
            .flat_map(|tile| tile[quadrant].to_vram_tilemap_word().to_le_bytes())
            .collect()
    })
}

fn split_map16_properties(tiles16: &[Tile16], tile_types: &[u8]) -> [Vec<u8>; 4] {
    std::array::from_fn(|quadrant| {
        let mut properties = vec![0; 0x4000];
        for (property, tile) in properties.iter_mut().zip(tiles16) {
            let word = tile[quadrant].to_vram_tilemap_word();
            let tile_type = tile_types[(word & 0x01ff) as usize];
            // Vanilla folds horizontal flip into the directional slope types.
            *property = if (0x10..0x1c).contains(&tile_type) {
                tile_type | ((word >> 14) as u8 & 1)
            } else {
                tile_type
            };
        }
        properties
    })
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut rom = fs::read(&args.input_rom)
        .with_context(|| format!("failed to read {}", args.input_rom.display()))?;
    anyhow::ensure!(
        rom.len() == 1024 * 1024,
        "expected a 1 MiB vanilla ROM, got {} bytes",
        rom.len()
    );
    let digest = Sha256::digest(&rom)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    anyhow::ensure!(
        digest == VANILLA_ROM_SHA256,
        "input ROM SHA-256 mismatch: expected {VANILLA_ROM_SHA256}, got {digest}"
    );
    let mut importer = Importer::new(rom.clone())?;
    let flat_map16 = importer.flat_map16()?;
    let tile_types = importer.tile_types()?.to_owned();
    let map16_definitions = split_map16_definitions(importer.tiles16()?);
    let map16_properties = split_map16_properties(importer.tiles16()?, &tile_types);
    rom.resize(2 * 1024 * 1024, 0);

    let mut patcher = Patcher::default();
    let mut context = patcher.context("flat Map16 data");
    context.write(FLAT_MAPS_START.into(), flat_map16.maps)?;
    context.write(FLAT_MAP_POINTERS_START.into(), flat_map16.screen_pointers)?;

    let mut context = patcher.context("expanded Map16 definitions");
    for (start, definitions) in MAP16_DEFINITION_STARTS.into_iter().zip(map16_definitions) {
        context.write(start.into(), definitions)?;
    }

    let mut context = patcher.context("Map16 quadrant properties");
    for (start, properties) in MAP16_PROPERTY_STARTS.into_iter().zip(map16_properties) {
        context.write(start.into(), properties)?;
    }

    // Zero out obsolete map data. This is only temporary, to prove that it is
    // no longer used.
    let mut context = patcher.context("zero obsolete map data");
    for range in [
        SnesAddr(0x0f8000)..SnesAddr(0x0ff4f0), // Map16 definitions
        SnesAddr(0x0ffd94)..SnesAddr(0x0fff94), // graphics-indexed tile properties
        SnesAddr(0x1bf110)..SnesAddr(0x1bffb0), // coarse Map16 properties
        SnesAddr(0x02f6b1)..SnesAddr(0x02fa71), // map pointer tables
        SnesAddr(0x038000)..SnesAddr(0x03e780), // top Tile32 definitions
        SnesAddr(0x048000)..SnesAddr(0x04e780), // bottom Tile32 definitions
        SnesAddr(0x0b8000)..SnesAddr(0x0bfe49), // compressed maps
        SnesAddr(0x0c8000)..SnesAddr(0x0cc0ab), // compressed maps
    ] {
        let start: PcAddr = range.start.into();
        let end: PcAddr = range.end.into();
        context.write(start, vec![0; (end.0 - start.0) as usize])?;
    }

    let patch_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../patches/ips");
    let patches = [
        "expand.ips",
        "flat_map16.ips",
        "expand_map16.ips",
        "independent_tile_type.ips",
        "reduce_bg3.ips",
        "bg_streamer.ips",
        "nmi_optimize.ips",
        "mirror_bg1.ips",
    ];
    for patch in patches {
        patcher.use_ips(&patch_dir.join(patch))?;
    }
    patcher.apply(&mut rom)?;

    fs::write(&args.output_rom, rom)
        .with_context(|| format!("failed to write {}", args.output_rom.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use patcher::import::Tile8;

    fn bg1_stream_bounds(hofs: u16, vofs: u16) -> [u16; 4] {
        [
            (hofs >> 3).wrapping_sub(1),
            (hofs.wrapping_add(255) >> 3).wrapping_add(1),
            (vofs >> 3).wrapping_sub(1),
            (vofs.wrapping_add(223) >> 3).wrapping_add(1),
        ]
    }

    fn bg1_stream_vram(tile_x: u16, tile_y: u16) -> u16 {
        0x1000 + (tile_y & 31) * 32 + (tile_x & 31) + (tile_x & 32) * 32
    }

    fn bg1_logical_origin(hofs: u16, vofs: u16) -> (u16, u16) {
        (hofs >> 3 & 0x3f, vofs >> 3 & 0x3f)
    }

    fn bg1_stream_delta_is_incremental(current: u16, resident: u16) -> bool {
        matches!(current.wrapping_sub(resident), 0 | 1 | u16::MAX)
    }

    fn bg1_uses_streamer(module: u8) -> bool {
        matches!(module, 0x08..=0x0b | 0x0e | 0x15)
    }

    fn bg2_stream_vram(tile_x: u16, tile_y: u16) -> u16 {
        (tile_y & 31) * 32 + (tile_x & 31) + (tile_x & 32) * 32
    }

    fn bg2_map16_vram(map_offset: u16) -> u16 {
        (map_offset & 0x20) * 32 + (map_offset & 0x1f) + (map_offset & 0x0780) / 2
    }

    fn bg2_logical_origin(screen: u8, hofs: u16, vofs: u16) -> (u16, u16) {
        let screen_x = u16::from(screen & 7) * 0x200;
        let screen_y = u16::from(screen & 0x38) * 0x40;
        (
            hofs.wrapping_sub(screen_x) >> 3 & 0x7f,
            vofs.wrapping_sub(screen_y) >> 3 & 0x7f,
        )
    }

    fn bg2_bulk_row_segments(tile_x: u16, tile_y: u16) -> [(u16, u16); 2] {
        let first_len = 32 - (tile_x & 31);
        [
            (bg2_stream_vram(tile_x, tile_y), first_len),
            (
                bg2_stream_vram(tile_x.wrapping_add(first_len), tile_y),
                33 - first_len,
            ),
        ]
    }

    fn bg2_column_segments(tile_x: u16, tile_y: u16) -> [(u16, u16); 2] {
        let first_len = (32 - (tile_y & 31)).min(29);
        [
            (bg2_stream_vram(tile_x, tile_y), first_len),
            (
                bg2_stream_vram(tile_x, tile_y.wrapping_add(first_len)),
                29 - first_len,
            ),
        ]
    }

    fn paired_quadrants(start: u16, len: u16) -> Vec<u16> {
        let mut quadrants = Vec::new();
        let mut remaining = len;
        if start & 1 != 0 {
            quadrants.push(1);
            remaining -= 1;
        }
        for _ in 0..remaining / 2 {
            quadrants.extend([0, 1]);
        }
        if remaining & 1 != 0 {
            quadrants.push(0);
        }
        quadrants
    }

    fn bg2_vertical_stream_rows(old_vofs: u16, new_vofs: u16) -> Option<(u16, u16)> {
        match (new_vofs >> 3).wrapping_sub(old_vofs >> 3) {
            1 => Some((28, 1)),
            u16::MAX => Some((0, 1)),
            0xfffe => Some((0, 2)),
            _ => None,
        }
    }

    fn bg2_transition_seed(direction: u8, left: u16, top: u16) -> Option<(u16, u16)> {
        match direction {
            1 => Some((left.wrapping_add(32) & 0x7f, top)),
            _ => None,
        }
    }

    #[test]
    fn bg1_stream_window_and_ring_geometry() {
        assert_eq!(bg1_logical_origin(0x0600, 0x06c0), (0, 24));
        assert_eq!(bg1_logical_origin(0xffff, 0xffff), (63, 63));

        assert_eq!(bg1_stream_bounds(0x100, 0x200), [31, 64, 63, 92]);
        assert_eq!(bg1_stream_bounds(0x101, 0x201), [31, 65, 63, 93]);

        assert_eq!(bg1_stream_vram(31, 31), 0x13ff);
        assert_eq!(bg1_stream_vram(32, 31), 0x17e0);
        assert_eq!(bg1_stream_vram(63, 31), 0x17ff);
        assert_eq!(bg1_stream_vram(64, 32), 0x1000);

        assert!(bg1_stream_delta_is_incremental(31, 32));
        assert!(bg1_stream_delta_is_incremental(33, 32));
        assert!(!bg1_stream_delta_is_incremental(34, 32));

        assert!(
            [0x08, 0x09, 0x0a, 0x0b, 0x0e, 0x15]
                .into_iter()
                .all(bg1_uses_streamer)
        );
        assert!(
            [0x07, 0x0c, 0x18, 0x19, 0x1a]
                .into_iter()
                .all(|module| !bg1_uses_streamer(module))
        );

        let row = 33 * 2 + 2 * 4;
        let split_column = 29 * 2 + 2 * 4;
        let after_bg2 = 2 * row + split_column;
        let after_bg1_horizontal = after_bg2 + split_column;
        let complete_list = after_bg1_horizontal + row + 2;

        assert_eq!(after_bg2, 214);
        assert!(after_bg2 <= u8::MAX as usize);
        assert!(after_bg1_horizontal > u8::MAX as usize);
        assert_eq!(complete_list, 356);
        assert!(complete_list <= 0x1980 - 0x1100);
    }

    #[test]
    fn bg2_bulk_rows_split_and_fit_the_dma_buffer() {
        // Link's house is screen $2C at world pixel origin ($800, $A00).
        assert_eq!(bg2_logical_origin(0x2c, 0x0832, 0x0a9a), (6, 19));

        assert_eq!(bg2_bulk_row_segments(0, 0), [(0x0000, 32), (0x0400, 1)]);
        assert_eq!(bg2_bulk_row_segments(31, 0), [(0x001f, 1), (0x0400, 32)]);
        assert_eq!(bg2_bulk_row_segments(32, 0), [(0x0400, 32), (0x0000, 1)]);
        assert_eq!(bg2_bulk_row_segments(63, 31), [(0x07ff, 1), (0x03e0, 32)]);

        for tile_x in 0..128 {
            let first_len = 32 - (tile_x & 31);
            for (start, len) in [(tile_x, first_len), (tile_x + first_len, 33 - first_len)] {
                assert_eq!(
                    paired_quadrants(start, len),
                    (0..len)
                        .map(|offset| (start + offset) & 1)
                        .collect::<Vec<_>>()
                );
            }
        }

        let list_size = 29 * (33 * 2 + 2 * 4) + 2;
        assert_eq!(list_size, 2148);
        assert!(list_size <= 0x1980 - 0x1100);

        let mirror_batches = [15, 14];
        assert_eq!(mirror_batches.iter().sum::<usize>(), 29);
        assert!(
            mirror_batches
                .map(|rows| rows * (33 * 2 + 2 * 4) + 2)
                .into_iter()
                .all(|size| size <= 0x1980 - 0x1100)
        );
    }

    #[test]
    fn bg2_vertical_stream_selects_only_the_entering_rows() {
        assert_eq!(bg2_vertical_stream_rows(0x0107, 0x0108), Some((28, 1)));
        assert_eq!(bg2_vertical_stream_rows(0x0100, 0x0110), None);
        assert_eq!(bg2_vertical_stream_rows(0x0108, 0x0107), Some((0, 1)));
        assert_eq!(bg2_vertical_stream_rows(0x0120, 0x0116), Some((0, 2)));
        assert_eq!(bg2_vertical_stream_rows(0x0101, 0x0107), None);
        assert_eq!(bg2_vertical_stream_rows(0x0100, 0x0118), None);

        let list_size = 2 * (33 * 2 + 2 * 4) + 2;
        assert_eq!(list_size, 150);
        assert!(list_size <= 0x1980 - 0x1100);
    }

    #[test]
    fn bg2_horizontal_stream_splits_columns_at_the_row_wrap() {
        assert_eq!(bg2_column_segments(0, 3)[0], (0x0060, 29));
        assert_eq!(bg2_column_segments(0, 4), [(0x0080, 28), (0x0000, 1)]);
        assert_eq!(bg2_column_segments(0, 31), [(0x03e0, 1), (0x0000, 28)]);
        assert_eq!(bg2_column_segments(32, 31), [(0x07e0, 1), (0x0400, 28)]);

        for tile_y in 0..128 {
            let first_len = (32 - (tile_y & 31)).min(29);
            for (start, len) in [(tile_y, first_len), (tile_y + first_len, 29 - first_len)] {
                if len == 0 {
                    continue;
                }
                assert_eq!(
                    paired_quadrants(start, len),
                    (0..len)
                        .map(|offset| (start + offset) & 1)
                        .collect::<Vec<_>>()
                );
            }
        }

        let two_rows_and_split_column = 2 * (33 * 2 + 2 * 4) + (29 * 2 + 2 * 4) + 2;
        assert_eq!(two_rows_and_split_column, 216);
        assert!(two_rows_and_split_column <= 0x1980 - 0x1100);

        assert_eq!(bg2_transition_seed(1, 96, 12), Some((0, 12)));
        assert_eq!(bg2_transition_seed(4, 7, 99), None);
        assert_eq!(bg2_transition_seed(2, 96, 12), None);
        assert_eq!(bg2_transition_seed(8, 7, 99), None);

        for subpixel in 0_i32..8 {
            for displacement in -9..=9 {
                let first = (subpixel + displacement).div_euclid(8);
                let last = (subpixel + displacement + 255).div_euclid(8);
                assert!(first >= -2);
                assert!(last <= 33);
            }
        }
    }

    #[test]
    fn bg2_map16_updates_use_the_tilemap_ring() {
        for row in 0..64 {
            for column in 0..64 {
                let map_offset = row * 0x80 + column * 2;
                assert_eq!(
                    bg2_map16_vram(map_offset),
                    bg2_stream_vram(column * 2, row * 2)
                );
            }
        }

        assert_eq!(bg2_map16_vram(15 * 2), 0x001e);
        assert_eq!(bg2_map16_vram(16 * 2), 0x0400);
        assert_eq!(bg2_map16_vram(15 * 0x80), 0x03c0);
        assert_eq!(bg2_map16_vram(16 * 0x80), 0x0000);
    }

    #[test]
    fn splits_map16_definitions_by_quadrant() {
        let tiles16 = [
            [0x0201, 0x0403, 0x0605, 0x0807].map(Tile8::from_vram_tilemap_word),
            [0x0A09, 0x0C0B, 0x0E0D, 0x100F].map(Tile8::from_vram_tilemap_word),
        ];

        assert_eq!(
            split_map16_definitions(&tiles16),
            [
                vec![1, 2, 9, 10],
                vec![3, 4, 11, 12],
                vec![5, 6, 13, 14],
                vec![7, 8, 15, 16],
            ]
        );
    }

    #[test]
    fn splits_map16_properties_by_quadrant() {
        let tiles16 = [
            [0xc201, 0x8201, 2, 3].map(Tile8::from_vram_tilemap_word),
            [4, 5, 6, 7].map(Tile8::from_vram_tilemap_word),
        ];
        let mut tile_types: Vec<_> = (0..=255).chain(0..=255).collect();
        tile_types[1] = 0x10;
        let properties = split_map16_properties(&tiles16, &tile_types);

        assert_eq!(&properties[0][..2], &[0x11, 4]);
        assert_eq!(&properties[1][..2], &[0x10, 5]);
        assert_eq!(&properties[2][..2], &[2, 6]);
        assert_eq!(&properties[3][..2], &[3, 7]);
        assert!(
            properties
                .iter()
                .all(|quadrant| quadrant[2..].iter().all(|&property| property == 0))
        );
    }
}
