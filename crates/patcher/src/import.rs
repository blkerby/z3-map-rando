// Based on https://github.com/blkerby/Z3OverworldEditor/blob/main/src/import.rs
#![allow(dead_code)]

use anyhow::{Result, bail, ensure};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Add, AddAssign},
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PcAddr(u32);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SnesAddr(u32);

macro_rules! impl_add {
    ($target_type:ident, $other_type:ident) => {
        impl Add<$other_type> for $target_type {
            type Output = $target_type;

            fn add(self, other: $other_type) -> Self {
                $target_type((self.0 as $other_type + other) as u32)
            }
        }
    };
}

impl_add!(PcAddr, u32);
impl_add!(SnesAddr, u32);

macro_rules! impl_add_assign {
    ($target_type:ident, $other_type:ident) => {
        impl AddAssign<$other_type> for $target_type {
            fn add_assign(&mut self, other: $other_type) {
                self.0 = (self.0 as $other_type + other) as u32;
            }
        }
    };
}

impl_add_assign!(PcAddr, u32);
impl_add_assign!(SnesAddr, u32);

impl Display for PcAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:X}", self.0)
    }
}

impl Display for SnesAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${:X}", self.0)
    }
}

impl SnesAddr {
    fn from_bytes(bank: u8, high: u8, low: u8) -> Self {
        Self((bank as u32) << 16 | (high as u32) << 8 | low as u32)
    }

    fn from_bank_offset(bank: u8, offset: u16) -> Self {
        Self((bank as u32) << 16 | offset as u32)
    }
}

impl From<SnesAddr> for PcAddr {
    fn from(addr: SnesAddr) -> Self {
        assert!(addr.0 & 0x8000 == 0x8000);
        PcAddr(addr.0 >> 1 & 0x3f8000 | addr.0 & 0x7fff)
    }
}

struct Constants {
    hud_palettes_addr: SnesAddr,
    main_palettes_addr: SnesAddr,
    aux_palettes_addr: SnesAddr,
    animated_palettes_addr: SnesAddr,
    gfx_bank_addr: SnesAddr,
    gfx_high_addr: SnesAddr,
    gfx_low_addr: SnesAddr,
    sprite_gfx_bank_addr: SnesAddr,
    sprite_gfx_high_addr: SnesAddr,
    sprite_gfx_low_addr: SnesAddr,
    sprite_sets_addr: SnesAddr,
    sprite_sheet_sets_addr: SnesAddr,
    special_sprite_sets_addr: SnesAddr,
    tiles16_addr: SnesAddr,
    tiles16_cnt: u32,
    tiles32_tl_addr: SnesAddr,
    tiles32_tr_addr: SnesAddr,
    tiles32_bl_addr: SnesAddr,
    tiles32_br_addr: SnesAddr,
    tiles32_cnt: u32,
    map_high_addr: SnesAddr,
    map_low_addr: SnesAddr,
    map_cnt: u32,
    custom_map_main_pal_set_addr: Option<SnesAddr>,
    map_aux_pal_set_addr: SnesAddr,
    special_map_pal_set_addr: SnesAddr,
    pal_set_addr: SnesAddr,
    global_gfx_set_addr: SnesAddr,
    local_gfx_set_addr: SnesAddr,
    map_gfx_set_addr: SnesAddr,
    custom_gfx_set_addr: Option<SnesAddr>,
    special_gfx_set_addr: SnesAddr,
    tile_types: SnesAddr,
    custom_bg_colors_addr: Option<SnesAddr>,
}

impl Constants {
    fn jp() -> Self {
        Self {
            hud_palettes_addr: SnesAddr(0x1bd660),
            main_palettes_addr: SnesAddr(0x1be6c8),
            aux_palettes_addr: SnesAddr(0x1be86c),
            animated_palettes_addr: SnesAddr(0x1be604),
            gfx_bank_addr: SnesAddr(0x00e7d0),
            gfx_high_addr: SnesAddr(0x00e7d5),
            gfx_low_addr: SnesAddr(0x00e7da),
            sprite_gfx_bank_addr: SnesAddr(0x00d033),
            sprite_gfx_high_addr: SnesAddr(0x00d112),
            sprite_gfx_low_addr: SnesAddr(0x00d1f1),
            sprite_sets_addr: SnesAddr(0x00fa41),
            sprite_sheet_sets_addr: SnesAddr(0x00db97),
            special_sprite_sets_addr: SnesAddr(0x02e575),
            tiles16_addr: SnesAddr(0x0f8000),
            tiles16_cnt: 3742,
            tiles32_tl_addr: SnesAddr(0x038000),
            tiles32_tr_addr: SnesAddr(0x03b3c0),
            tiles32_bl_addr: SnesAddr(0x048000),
            tiles32_br_addr: SnesAddr(0x04b3c0),
            tiles32_cnt: 8828,
            map_high_addr: SnesAddr(0x02f6b1),
            map_low_addr: SnesAddr(0x02f891),
            map_cnt: 0xa0,
            custom_map_main_pal_set_addr: None,
            map_aux_pal_set_addr: SnesAddr(0x00fd1c),
            special_map_pal_set_addr: SnesAddr(0x02e595),
            pal_set_addr: SnesAddr(0x0cfe74),
            global_gfx_set_addr: SnesAddr(0x00e0b3),
            local_gfx_set_addr: SnesAddr(0x00ddd7),
            map_gfx_set_addr: SnesAddr(0x00fc9c),
            custom_gfx_set_addr: None,
            special_gfx_set_addr: SnesAddr(0x02e585),
            tile_types: SnesAddr(0x0ffd94),
            custom_bg_colors_addr: None,
        }
    }

    fn us() -> Self {
        Self {
            hud_palettes_addr: SnesAddr(0x1bd660),
            main_palettes_addr: SnesAddr(0x1be6c8),
            aux_palettes_addr: SnesAddr(0x1be86c),
            animated_palettes_addr: SnesAddr(0x1be604),
            gfx_bank_addr: SnesAddr(0x00e790),
            gfx_high_addr: SnesAddr(0x00e795),
            gfx_low_addr: SnesAddr(0x00e79a),
            sprite_gfx_bank_addr: SnesAddr(0x00cff3),
            sprite_gfx_high_addr: SnesAddr(0x00d0d2),
            sprite_gfx_low_addr: SnesAddr(0x00d1b1),
            sprite_sets_addr: SnesAddr(0x00fa41),
            sprite_sheet_sets_addr: SnesAddr(0x00db57),
            special_sprite_sets_addr: SnesAddr(0x02e811),
            tiles16_addr: SnesAddr(0x0f8000),
            tiles16_cnt: 3742,
            tiles32_tl_addr: SnesAddr(0x038000),
            tiles32_tr_addr: SnesAddr(0x03b400),
            tiles32_bl_addr: SnesAddr(0x048000),
            tiles32_br_addr: SnesAddr(0x04b400),
            tiles32_cnt: 8864,
            map_high_addr: SnesAddr(0x02f94d),
            map_low_addr: SnesAddr(0x02fb2d),
            map_cnt: 0xa0,
            custom_map_main_pal_set_addr: None,
            map_aux_pal_set_addr: SnesAddr(0x00fd1c),
            special_map_pal_set_addr: SnesAddr(0x02e831),
            pal_set_addr: SnesAddr(0x0ed504),
            global_gfx_set_addr: SnesAddr(0x00e073),
            local_gfx_set_addr: SnesAddr(0x00dd97),
            map_gfx_set_addr: SnesAddr(0x00fc9c),
            custom_gfx_set_addr: None,
            special_gfx_set_addr: SnesAddr(0x02e821),
            tile_types: SnesAddr(0x0e9459),
            custom_bg_colors_addr: None,
        }
    }

    fn auto(rom: &Rom) -> Result<Self> {
        if rom.read_u24(SnesAddr(0x008865).into())? == 0xbd8000 {
            let mut constants = Self::us();
            constants.tiles16_addr = SnesAddr(0xbd8000);
            constants.tiles16_cnt = 4096;
            constants.tiles32_tr_addr = SnesAddr(0x048000);
            constants.tiles32_bl_addr = SnesAddr(0x3e8000);
            constants.tiles32_br_addr = SnesAddr(0x3f8000);
            constants.tiles32_cnt = 17728;

            if rom.read_u8(SnesAddr(0x288148).into())? != 0 {
                constants.custom_gfx_set_addr = Some(SnesAddr(0x288480));
            }
            if rom.read_u8(SnesAddr(0x288141).into())? != 0 {
                constants.custom_map_main_pal_set_addr = Some(SnesAddr(0x288160));
            }
            if rom.read_u8(SnesAddr(0x288140).into())? != 0 {
                constants.custom_bg_colors_addr = Some(SnesAddr(0x288000));
            }
            Ok(constants)
        } else if rom.read_u16(SnesAddr(0x00e7d2).into())? == 0xca85 {
            Ok(Self::jp())
        } else if rom.read_u16(SnesAddr(0x00e792).into())? == 0xca85 {
            Ok(Self::us())
        } else {
            bail!("Unknown ROM format.")
        }
    }
}

#[derive(Clone)]
struct Rom {
    data: Vec<u8>,
}

impl Rom {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    fn read_u8(&self, addr: PcAddr) -> Result<u8> {
        ensure!(
            (addr.0 as usize) < self.data.len(),
            "read_u8 address out of bounds"
        );
        Ok(self.data[addr.0 as usize])
    }

    fn read_u16(&self, addr: PcAddr) -> Result<u16> {
        ensure!(
            addr.0 as usize + 1 < self.data.len(),
            "read_u16 address out of bounds"
        );
        Ok(u16::from_le_bytes([
            self.data[addr.0 as usize],
            self.data[addr.0 as usize + 1],
        ]))
    }

    fn read_u24(&self, addr: PcAddr) -> Result<u32> {
        ensure!(
            addr.0 as usize + 2 < self.data.len(),
            "read_u24 address out of bounds"
        );
        Ok(u32::from_le_bytes([
            self.data[addr.0 as usize],
            self.data[addr.0 as usize + 1],
            self.data[addr.0 as usize + 2],
            0,
        ]))
    }

    fn read_n(&self, addr: PcAddr, n: usize) -> Result<&[u8]> {
        ensure!(
            addr.0 as usize + n <= self.data.len(),
            "read_n address out of bounds"
        );
        Ok(&self.data[addr.0 as usize..addr.0 as usize + n])
    }
}

#[derive(Copy, Clone, Debug)]
enum Flip {
    None,
    Horizontal,
    Vertical,
    Both,
}

#[derive(Copy, Clone, Debug)]
pub struct Tile8 {
    gfx_char: u16,
    pal_idx: u8,
    priority: bool,
    flip: Flip,
}

impl Tile8 {
    pub fn from_vram_tilemap_word(word: u16) -> Self {
        Self {
            gfx_char: word & 0x3ff,
            pal_idx: ((word >> 10) & 7) as u8,
            priority: (word >> 13) & 1 == 1,
            flip: match word >> 14 {
                0 => Flip::None,
                1 => Flip::Horizontal,
                2 => Flip::Vertical,
                3 => Flip::Both,
                _ => unreachable!(),
            },
        }
    }

    pub fn to_vram_tilemap_word(self) -> u16 {
        self.gfx_char
            | u16::from(self.pal_idx) << 10
            | u16::from(self.priority) << 13
            | match self.flip {
                Flip::None => 0,
                Flip::Horizontal => 1 << 14,
                Flip::Vertical => 2 << 14,
                Flip::Both => 3 << 14,
            }
    }
}

pub type Tile16 = [Tile8; 4];
type Tile16Idx = u16;
type Tile32 = [Tile16Idx; 4];
type Tile32Idx = u16;
type MapIdx = u16;
type ColorRgb = [u8; 3];

#[derive(Debug)]
struct MapPalettes {
    main: u8,
    aux1: u8,
    aux2: u8,
    animated: u8,
    transition_groups: [bool; 2],
}

pub struct Importer {
    constants: Constants,
    rom: Rom,
    palettes: Vec<[ColorRgb; 16]>,
    tiles8: Vec<[[u8; 8]; 8]>,
    tiles16: Vec<Tile16>,
    tiles32: Vec<Tile32>,
    map_tiles: Vec<[[Tile32Idx; 16]; 16]>,
    map_pointers: Vec<(u32, u32)>,
    map_parents: Vec<MapIdx>,
    map_palettes: Vec<MapPalettes>,
    map_gfx: Vec<[u8; 8]>,
    map_gfx_transition_sheets: Vec<[bool; 8]>,
    sprite_character_rows: HashMap<u8, Vec<[u8; 512]>>,
    tile_types: Vec<u8>,
}

pub struct FlatMap16 {
    pub maps: Vec<u8>,
    pub screen_pointers: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct OverworldSpriteVariant {
    pub max_game_state: u8,
    pub character_rows: Vec<(u8, [u8; 512])>,
}

#[derive(Clone)]
pub struct OverworldAreaAssets {
    pub palette_rows: Vec<[u8; 32]>,
    pub character_rows: Vec<[u8; 512]>,
    pub transition_palette_rows: Vec<bool>,
    pub transition_character_rows: Vec<bool>,
    pub sprite_variants: Vec<OverworldSpriteVariant>,
}

impl Importer {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        let rom = Rom::new(data);
        Ok(Self {
            constants: Constants::auto(&rom)?,
            rom,
            palettes: Vec::new(),
            tiles8: Vec::new(),
            tiles16: Vec::new(),
            tiles32: Vec::new(),
            map_tiles: Vec::new(),
            map_pointers: Vec::new(),
            map_parents: Vec::new(),
            map_palettes: Vec::new(),
            map_gfx: Vec::new(),
            map_gfx_transition_sheets: Vec::new(),
            sprite_character_rows: HashMap::new(),
            tile_types: Vec::new(),
        })
    }

    fn import_all(&mut self) -> Result<()> {
        self.load_palettes()?;
        self.load_graphics()?;
        self.load_16x16_tiles()?;
        self.load_32x32_tiles()?;
        self.load_map_tiles()?;
        self.load_map_parents();
        self.load_map_palettes()?;
        self.load_map_gfx()?;
        self.load_tile_types()?;
        Ok(())
    }

    pub fn flat_map16(&mut self, first_bank: u8) -> Result<FlatMap16> {
        if self.tiles32.is_empty() {
            self.load_32x32_tiles()?;
        }
        if self.map_tiles.is_empty() {
            self.load_map_tiles()?;
        }

        let mut indices = HashMap::new();
        let mut maps = Vec::with_capacity(124 * 2048);
        let mut screen_pointers = Vec::with_capacity(self.map_tiles.len() * 3);
        for (pointers, map) in self.map_pointers.iter().zip(&self.map_tiles) {
            let index = if let Some(&index) = indices.get(pointers) {
                index
            } else {
                let index = u8::try_from(indices.len()).expect("too many flat Map16 maps");
                indices.insert(*pointers, index);

                let mut flat = [0u16; 32 * 32];
                for (y, row) in map.iter().enumerate() {
                    for (x, &tile32) in row.iter().enumerate() {
                        let tile32 = self.tiles32[tile32 as usize];
                        let offset = y * 2 * 32 + x * 2;
                        flat[offset] = tile32[0];
                        flat[offset + 1] = tile32[1];
                        flat[offset + 32] = tile32[2];
                        flat[offset + 33] = tile32[3];
                    }
                }
                maps.extend(flat.into_iter().flat_map(u16::to_le_bytes));
                index
            };

            let pointer = (u32::from(first_bank + index / 16) << 16)
                | (0x8000 + u32::from(index % 16) * 0x0800);
            screen_pointers.extend_from_slice(&pointer.to_le_bytes()[..3]);
        }
        ensure!(
            indices.len() == 124,
            "expected 124 unique maps, found {}",
            indices.len()
        );
        ensure!(
            screen_pointers.len() == 160 * 3,
            "expected 160 screen pointers, found {}",
            screen_pointers.len() / 3
        );
        Ok(FlatMap16 {
            maps,
            screen_pointers,
        })
    }

    pub fn tiles16(&mut self) -> Result<&[Tile16]> {
        if self.tiles16.is_empty() {
            self.load_16x16_tiles()?;
        }
        Ok(&self.tiles16)
    }

    pub fn tile_types(&mut self) -> Result<&[u8]> {
        if self.tile_types.is_empty() {
            self.load_tile_types()?;
        }
        Ok(&self.tile_types)
    }

    pub fn overworld_area_assets(&mut self) -> Result<Vec<OverworldAreaAssets>> {
        if self.palettes.is_empty() {
            self.load_palettes()?;
        }
        if self.tiles8.is_empty() {
            self.load_graphics()?;
        }
        if self.map_parents.is_empty() {
            self.load_map_parents();
        }
        if self.map_palettes.is_empty() {
            self.load_map_palettes()?;
        }
        if self.map_gfx.is_empty() {
            self.load_map_gfx()?;
        }

        let mut assets = Vec::with_capacity(self.map_gfx.len());
        for area in 0..self.map_gfx.len() {
            let mut area_assets = self.encode_overworld_area_assets(
                &self.map_gfx[area],
                self.map_gfx_transition_sheets[area],
                &self.map_palettes[area],
            )?;
            area_assets.sprite_variants = self.overworld_sprite_variants(area)?;
            assets.push(area_assets);
        }
        Ok(assets)
    }

    pub fn credits_overworld_assets(&mut self) -> Result<Vec<(u8, OverworldAreaAssets)>> {
        let areas = self.overworld_area_assets()?;
        // Final $8A for each scripted credits scene; None marks a dungeon.
        let area_keys = [
            Some(0x1b),
            None,
            Some(0x18),
            Some(0x30),
            Some(0x03),
            Some(0x2c),
            Some(0x81),
            Some(0x16),
            Some(0x02),
            Some(0x2a),
            None,
            None,
            Some(0x18),
            Some(0x05),
            Some(0x00),
            Some(0x80),
        ];
        let sprite_sets = [
            0x28, 0x46, 0x27, 0x2e, 0x2b, 0x2b, 0x0e, 0x2c, 0x1a, 0x29, 0x47, 0x28, 0x27, 0x28,
            0x2a, 0x28,
        ];
        let mut credits = Vec::new();
        for (scene, area) in area_keys.into_iter().enumerate() {
            if let Some(area) = area {
                let mut assets = areas[area].clone();
                assets.sprite_variants = vec![self.sprite_variant(0xff, sprite_sets[scene])?];
                credits.push((u8::try_from(scene)?, assets));
            }
        }
        Ok(credits)
    }

    pub fn overworld_sprite_seed(&mut self) -> Result<OverworldSpriteVariant> {
        let rows = self.sprite_sheet_rows(0x46)?.clone();
        Ok(OverworldSpriteVariant {
            max_game_state: 0xff,
            character_rows: (0..4)
                .flat_map(|slot| {
                    rows.iter()
                        .cloned()
                        .enumerate()
                        .map(move |(row, data)| (0x50 + slot * 4 + row as u8, data))
                })
                .collect(),
        })
    }

    pub fn credits_cool_background_assets(&mut self) -> Result<OverworldAreaAssets> {
        if self.palettes.is_empty() {
            self.load_palettes()?;
        }
        if self.tiles8.is_empty() {
            self.load_graphics()?;
        }
        if self.map_parents.is_empty() {
            self.load_map_parents();
        }
        if self.map_palettes.is_empty() {
            self.load_map_palettes()?;
        }
        if self.map_gfx.is_empty() {
            self.load_map_gfx()?;
        }

        let area = 0x5b;
        let ordinary = &self.map_palettes[area];
        let palettes = MapPalettes {
            main: ordinary.main,
            aux1: ordinary.aux1,
            aux2: 3,
            animated: ordinary.animated,
            transition_groups: [false; 2],
        };
        let mut assets =
            self.encode_overworld_area_assets(&self.map_gfx[area], [false; 8], &palettes)?;
        assets.sprite_variants = vec![self.sprite_variant(0xff, 0x2d)?];
        Ok(assets)
    }

    fn encode_overworld_area_assets(
        &self,
        gfx: &[u8; 8],
        transition_sheets: [bool; 8],
        palettes: &MapPalettes,
    ) -> Result<OverworldAreaAssets> {
        let mut character_rows = Vec::with_capacity(32);
        for (slot, &sheet) in gfx.iter().enumerate() {
            let right_palette = matches!(slot, 0 | 3 | 4 | 5);
            let start = usize::from(sheet) * 64;
            ensure!(
                start + 64 <= self.tiles8.len(),
                "overworld graphics sheet {sheet:02X} is unavailable"
            );
            let tiles = &self.tiles8[start..start + 64];
            for tiles in tiles.chunks_exact(16) {
                let mut row = [0; 512];
                for (tile, output) in tiles.iter().zip(row.chunks_exact_mut(32)) {
                    encode_4bpp_tile(tile, right_palette, output);
                }
                character_rows.push(row);
            }
        }

        let mut palette_rows = vec![[0; 32]; 6];
        let main = 2 + usize::from(palettes.main) * 5;
        for (row, palette) in palette_rows[..5]
            .iter_mut()
            .zip(&self.palettes[main..main + 5])
        {
            encode_palette_half(palette, &mut row[..16]);
        }

        let aux1 = 32 + usize::from(palettes.aux1) * 3;
        for (row, palette) in palette_rows[..3]
            .iter_mut()
            .zip(&self.palettes[aux1..aux1 + 3])
        {
            encode_palette_half(palette, &mut row[16..]);
        }

        let aux2 = 32 + usize::from(palettes.aux2) * 3;
        for (row, palette) in palette_rows[3..]
            .iter_mut()
            .zip(&self.palettes[aux2..aux2 + 3])
        {
            encode_palette_half(palette, &mut row[16..]);
        }

        let animated = 92 + usize::from(palettes.animated);
        encode_palette_half(&self.palettes[animated], &mut palette_rows[5][..16]);

        let mut transition_character_rows = Vec::with_capacity(character_rows.len());
        for load in transition_sheets {
            for _ in 0..4 {
                transition_character_rows.push(load);
            }
        }
        transition_character_rows.truncate(character_rows.len());

        let mut transition_palette_rows = Vec::with_capacity(6);
        for load in palettes.transition_groups {
            for _ in 0..3 {
                transition_palette_rows.push(load);
            }
        }

        Ok(OverworldAreaAssets {
            palette_rows,
            character_rows,
            transition_palette_rows,
            transition_character_rows,
            sprite_variants: Vec::new(),
        })
    }

    fn overworld_sprite_variants(&mut self, area: usize) -> Result<Vec<OverworldSpriteVariant>> {
        let parent = usize::from(self.map_parents[area]);
        let variants = if area < 0x40 {
            // Three Light World progress blocks followed by the fixed Dark World block.
            [(0x01, area), (0x02, 0x40 + area), (0xff, 0x80 + area)]
                .into_iter()
                .map(|(max_state, index)| {
                    let set = self
                        .rom
                        .read_u8((self.constants.sprite_sets_addr + index as u32).into())?;
                    self.sprite_variant(max_state, set)
                })
                .collect::<Result<Vec<_>>>()?
        } else if area < 0x80 {
            let set = self
                .rom
                .read_u8((self.constants.sprite_sets_addr + 0xc0 + (area - 0x40) as u32).into())?;
            vec![self.sprite_variant(0xff, set)?]
        } else if parent == 0x88 {
            // Triforce's $0AA3=$7D row names raw common sheet $08, which its
            // later dedicated loader installs outside the area-dependent slots.
            vec![OverworldSpriteVariant {
                max_game_state: 0xff,
                character_rows: Vec::new(),
            }]
        } else {
            let set = self
                .rom
                .read_u8((self.constants.special_sprite_sets_addr + (area - 0x80) as u32).into())?;
            vec![self.sprite_variant(0xff, set)?]
        };

        let mut merged: Vec<OverworldSpriteVariant> = Vec::new();
        for variant in variants {
            if let Some(previous) = merged.last_mut()
                && previous.character_rows == variant.character_rows
            {
                previous.max_game_state = variant.max_game_state;
            } else {
                merged.push(variant);
            }
        }
        Ok(merged)
    }

    fn sprite_variant(&mut self, max_game_state: u8, set: u8) -> Result<OverworldSpriteVariant> {
        let mut character_rows = Vec::new();
        for slot in 0..4 {
            let sheet = self.rom.read_u8(
                (self.constants.sprite_sheet_sets_addr + u32::from(set) * 4 + u32::try_from(slot)?)
                    .into(),
            )?;
            if sheet == 0 {
                continue;
            }
            for (row, data) in self
                .sprite_sheet_rows(sheet)?
                .clone()
                .into_iter()
                .enumerate()
            {
                character_rows.push((0x50 + slot as u8 * 4 + row as u8, data));
            }
        }
        Ok(OverworldSpriteVariant {
            max_game_state,
            character_rows,
        })
    }

    fn sprite_sheet_rows(&mut self, sheet: u8) -> Result<&Vec<[u8; 512]>> {
        if !self.sprite_character_rows.contains_key(&sheet) {
            let bank = self
                .rom
                .read_u8((self.constants.sprite_gfx_bank_addr + u32::from(sheet)).into())?;
            let high = self
                .rom
                .read_u8((self.constants.sprite_gfx_high_addr + u32::from(sheet)).into())?;
            let low = self
                .rom
                .read_u8((self.constants.sprite_gfx_low_addr + u32::from(sheet)).into())?;
            let data = decompress(
                &self.rom,
                SnesAddr::from_bytes(bank, high, low).into(),
                false,
            )?;
            ensure!(
                data.len() == 0x600,
                "unexpected sprite graphics sheet {sheet:02X} length: {}",
                data.len()
            );
            let right_palette = matches!(sheet, 0x52 | 0x53 | 0x5a | 0x5b | 0x5c | 0x5e | 0x5f);
            let tiles = decode_3bpp_tiles(&data);
            let mut rows = Vec::with_capacity(4);
            for tiles in tiles.chunks_exact(16) {
                let mut row = [0; 512];
                for (tile, output) in tiles.iter().zip(row.chunks_exact_mut(32)) {
                    encode_4bpp_tile(tile, right_palette, output);
                }
                rows.push(row);
            }
            self.sprite_character_rows.insert(sheet, rows);
        }
        Ok(&self.sprite_character_rows[&sheet])
    }

    fn load_palette(&self, addr: PcAddr, size: usize) -> Result<[ColorRgb; 16]> {
        let mut colors = [[0, 0, 0]; 16];
        for i in 0..size {
            let color = self.rom.read_u16(addr + i as u32 * 2)?;
            colors[i + 1] = [
                (color & 31) as u8,
                ((color >> 5) & 31) as u8,
                ((color >> 10) & 31) as u8,
            ];
        }
        Ok(colors)
    }

    fn load_palettes(&mut self) -> Result<()> {
        for (base, count, rows, size) in [
            (self.constants.hud_palettes_addr, 1, 2, 15),
            (self.constants.main_palettes_addr, 6, 5, 7),
            (self.constants.aux_palettes_addr, 20, 3, 7),
            (self.constants.animated_palettes_addr, 14, 1, 7),
        ] {
            let base: PcAddr = base.into();
            for i in 0..count {
                for j in 0..rows {
                    self.palettes.push(
                        self.load_palette(base + ((i * rows + j) * size) * 2, size as usize)?,
                    );
                }
            }
        }
        Ok(())
    }

    fn load_graphics(&mut self) -> Result<()> {
        let gfx_bank = self.rom.read_u16(self.constants.gfx_bank_addr.into())?;
        let gfx_high = self.rom.read_u16(self.constants.gfx_high_addr.into())?;
        let gfx_low = self.rom.read_u16(self.constants.gfx_low_addr.into())?;

        // Map graphics select sheets through $72. The last two deliberately
        // pass 4bpp sprite data through vanilla's background conversion.
        for i in 0..=0x72 {
            let bank = self
                .rom
                .read_u8(SnesAddr::from_bank_offset(0, gfx_bank + i).into())?;
            let high = self
                .rom
                .read_u8(SnesAddr::from_bank_offset(0, gfx_high + i).into())?;
            let low = self
                .rom
                .read_u8(SnesAddr::from_bank_offset(0, gfx_low + i).into())?;
            let data = decompress(
                &self.rom,
                SnesAddr::from_bytes(bank, high, low).into(),
                false,
            )?;
            let expected_len = if i <= 0x70 { 0x600 } else { 0x800 };
            ensure!(
                data.len() == expected_len,
                "unexpected graphics sheet {i:02X} length: {}",
                data.len()
            );

            self.tiles8.extend(decode_3bpp_tiles(&data));
        }
        Ok(())
    }

    fn load_16x16_tiles(&mut self) -> Result<()> {
        for i in 0..self.constants.tiles16_cnt {
            let addr = self.constants.tiles16_addr + i * 8;
            self.tiles16.push([
                Tile8::from_vram_tilemap_word(self.rom.read_u16(addr.into())?),
                Tile8::from_vram_tilemap_word(self.rom.read_u16((addr + 2).into())?),
                Tile8::from_vram_tilemap_word(self.rom.read_u16((addr + 4).into())?),
                Tile8::from_vram_tilemap_word(self.rom.read_u16((addr + 6).into())?),
            ]);
        }
        Ok(())
    }

    fn load_32x32_tiles(&mut self) -> Result<()> {
        let bases: [PcAddr; 4] = [
            self.constants.tiles32_tl_addr.into(),
            self.constants.tiles32_tr_addr.into(),
            self.constants.tiles32_bl_addr.into(),
            self.constants.tiles32_br_addr.into(),
        ];
        let mut offset = 0;
        let size = self.constants.tiles32_cnt * 6 / 4;
        while offset < size {
            for i in 0..4 {
                let mut quadrants = [0; 4];
                for (quadrant, base) in bases.into_iter().enumerate() {
                    let addr = base + offset;
                    quadrants[quadrant] = match i {
                        0 => {
                            self.rom.read_u8(addr)? as u16
                                | (self.rom.read_u8(addr + 4)? as u16 & 0xf0) << 4
                        }
                        1 => {
                            self.rom.read_u8(addr + 1)? as u16
                                | (self.rom.read_u8(addr + 4)? as u16 & 0x0f) << 8
                        }
                        2 => {
                            self.rom.read_u8(addr + 2)? as u16
                                | (self.rom.read_u8(addr + 5)? as u16 & 0xf0) << 4
                        }
                        3 => {
                            self.rom.read_u8(addr + 3)? as u16
                                | (self.rom.read_u8(addr + 5)? as u16 & 0x0f) << 8
                        }
                        _ => unreachable!(),
                    };
                }
                self.tiles32.push(quadrants);
            }
            offset += 6;
        }
        Ok(())
    }

    fn load_map_tiles(&mut self) -> Result<()> {
        for i in 0..self.constants.map_cnt {
            let high_pointer = self
                .rom
                .read_u24((self.constants.map_high_addr + i * 3).into())?;
            let low_pointer = self
                .rom
                .read_u24((self.constants.map_low_addr + i * 3).into())?;
            let high = decompress(&self.rom, SnesAddr(high_pointer).into(), true)?;
            let low = decompress(&self.rom, SnesAddr(low_pointer).into(), true)?;
            ensure!(high.len() == 256 && low.len() == 256);

            let mut block = [[0; 16]; 16];
            for (y, row) in block.iter_mut().enumerate() {
                for (x, tile) in row.iter_mut().enumerate() {
                    let index = y * 16 + x;
                    let value = u16::from_be_bytes([high[index], low[index]]);
                    *tile = if u32::from(value) < self.constants.tiles32_cnt {
                        value
                    } else {
                        0
                    };
                }
            }
            self.map_pointers.push((high_pointer, low_pointer));
            self.map_tiles.push(block);
        }
        Ok(())
    }

    fn load_map_parents(&mut self) {
        let mut parents: Vec<MapIdx> = (0..self.constants.map_cnt as MapIdx).collect();
        for i in [0, 3, 5, 24, 27, 30, 48, 53] {
            for j in [0, 64] {
                parents[i + j + 1] = (i + j) as MapIdx;
                parents[i + j + 8] = (i + j) as MapIdx;
                parents[i + j + 9] = (i + j) as MapIdx;
            }
        }
        parents[130] = 129;
        parents[137] = 129;
        parents[138] = 129;
        self.map_parents = parents;
    }

    fn load_map_palettes(&mut self) -> Result<()> {
        for i in 0..self.constants.map_cnt as usize {
            let parent = self.map_parents[i];
            let main = if let Some(addr) = self.constants.custom_map_main_pal_set_addr {
                self.rom.read_u8((addr + u32::from(parent)).into())?
            } else {
                match i {
                    3 | 5 | 7 => 2,
                    0..0x40 => 0,
                    0x43 | 0x45 | 0x47 => 3,
                    0x40..0x80 => 1,
                    0x88 => 4,
                    0x80..0xa0 => 0,
                    _ => unreachable!(),
                }
            };
            let palette_set = if i == 0x88 {
                0x0e
            } else if parent >= 0x80 {
                self.rom.read_u8(
                    (self.constants.special_map_pal_set_addr + u32::from(parent - 0x80)).into(),
                )?
            } else {
                self.rom
                    .read_u8((self.constants.map_aux_pal_set_addr + u32::from(parent)).into())?
            };
            let addr = self.constants.pal_set_addr + u32::from(palette_set) * 4;
            let mut aux1 = self.rom.read_u8(addr.into())?;
            let mut aux2 = self.rom.read_u8((addr + 1).into())?;
            let mut animated = self.rom.read_u8((addr + 2).into())?;
            let transition_groups = [aux1 != 0xff, aux2 != 0xff];
            if aux1 >= 20 {
                aux1 = 0;
            }
            if aux2 >= 20 {
                aux2 = 0;
            }
            if animated >= 14 {
                animated = 0;
            }
            self.map_palettes.push(MapPalettes {
                main,
                aux1,
                aux2,
                animated,
                transition_groups,
            });
        }
        Ok(())
    }

    fn load_map_gfx(&mut self) -> Result<()> {
        for i in 0..self.constants.map_cnt as usize {
            let parent = self.map_parents[i];
            let global = match parent {
                0x40..0x80 => 0x21,
                0x88 => 0x24,
                _ => 0x20,
            };
            let mut transition_sheets = [false; 8];
            let mut gfx: Vec<u8> = self
                .rom
                .read_n((self.constants.global_gfx_set_addr + global * 8).into(), 8)?
                .to_owned();
            if let Some(addr) = self.constants.custom_gfx_set_addr {
                let local = self.rom.read_n((addr + u32::from(parent) * 8).into(), 8)?;
                for i in 0..8 {
                    if local[i] != 0xff {
                        gfx[i] = local[i];
                        transition_sheets[i] = true;
                    }
                }
            } else {
                let local = match parent {
                    0x88 => 81,
                    0x80.. => self.rom.read_u8(
                        (self.constants.special_gfx_set_addr + u32::from(parent - 0x80)).into(),
                    )?,
                    _ => self
                        .rom
                        .read_u8((self.constants.map_gfx_set_addr + u32::from(parent)).into())?,
                };
                let local = self.rom.read_n(
                    (self.constants.local_gfx_set_addr + u32::from(local) * 4).into(),
                    4,
                )?;
                for j in 0..4 {
                    if local[j] != 0 {
                        gfx[3 + j] = local[j];
                        transition_sheets[3 + j] = true;
                    }
                }
            }
            self.map_gfx.push(gfx.try_into().unwrap());
            self.map_gfx_transition_sheets.push(transition_sheets);
        }
        Ok(())
    }

    fn load_tile_types(&mut self) -> Result<()> {
        self.tile_types = self
            .rom
            .read_n(self.constants.tile_types.into(), 512)?
            .to_owned();
        Ok(())
    }
}

fn encode_palette_half(palette: &[ColorRgb; 16], output: &mut [u8]) {
    for (&[red, green, blue], output) in palette[1..8].iter().zip(output[2..].chunks_exact_mut(2)) {
        output.copy_from_slice(
            &(u16::from(red) | u16::from(green) << 5 | u16::from(blue) << 10).to_le_bytes(),
        );
    }
}

fn decode_3bpp_tiles(data: &[u8]) -> Vec<[[u8; 8]; 8]> {
    (0..64)
        .map(|tile_index| {
            let mut tile = [[0; 8]; 8];
            for (y, row) in tile.iter_mut().enumerate() {
                for (x, pixel) in row.iter_mut().enumerate() {
                    let c0 = (data[tile_index * 24 + y * 2] >> (7 - x)) & 1;
                    let c1 = (data[tile_index * 24 + y * 2 + 1] >> (7 - x)) & 1;
                    let c2 = (data[tile_index * 24 + y + 16] >> (7 - x)) & 1;
                    *pixel = c0 | c1 << 1 | c2 << 2;
                }
            }
            tile
        })
        .collect()
}

fn encode_4bpp_tile(tile: &[[u8; 8]; 8], right_palette: bool, output: &mut [u8]) {
    for (y, pixels) in tile.iter().enumerate() {
        for (x, &pixel) in pixels.iter().enumerate() {
            let pixel = pixel | u8::from(right_palette && pixel != 0) << 3;
            let mask = 0x80 >> x;
            output[y * 2] |= (pixel & 1 != 0) as u8 * mask;
            output[y * 2 + 1] |= (pixel & 2 != 0) as u8 * mask;
            output[16 + y * 2] |= (pixel & 4 != 0) as u8 * mask;
            output[16 + y * 2 + 1] |= (pixel & 8 != 0) as u8 * mask;
        }
    }
}

fn decompress(rom: &Rom, mut addr: PcAddr, big_endian_offset: bool) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let byte = rom.read_u8(addr)? as isize;
        addr += 1;
        if byte == 0xff {
            return Ok(out);
        }
        let mut block_type = byte >> 5;
        let size;
        if block_type != 7 {
            size = ((byte & 0x1f) + 1) as usize;
        } else {
            size = (((byte & 3) << 8 | rom.read_u8(addr)? as isize) + 1) as usize;
            addr += 1;
            block_type = byte >> 2 & 7;
        }

        match block_type {
            0 => {
                out.extend(rom.read_n(addr, size)?);
                addr += size as u32;
            }
            1 => {
                let value = rom.read_u8(addr)?;
                addr += 1;
                out.resize(out.len() + size, value);
            }
            2 => {
                let values = [rom.read_u8(addr)?, rom.read_u8(addr + 1)?];
                addr += 2;
                out.extend((0..size).map(|i| values[i % 2]));
            }
            3 => {
                let value = rom.read_u8(addr)?;
                addr += 1;
                out.extend((0..size).map(|i| value.wrapping_add(i as u8)));
            }
            4 => {
                let offset = if big_endian_offset {
                    usize::from(rom.read_u8(addr)?) << 8 | usize::from(rom.read_u8(addr + 1)?)
                } else {
                    usize::from(rom.read_u16(addr)?)
                };
                ensure!(offset < out.len(), "invalid compressed-data back-reference");
                addr += 2;
                for i in offset..offset + size {
                    out.push(out[i]);
                }
            }
            _ => bail!("Unexpected/impossible block type: {block_type}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompresses_map_data() {
        let rom = Rom::new(vec![
            0x02, 1, 2, 3, 0x21, 4, 0x43, 0xaa, 0xbb, 0x62, 0x10, 0x83, 0, 2, 0xff,
        ]);
        assert_eq!(
            decompress(&rom, PcAddr(0), true).unwrap(),
            [
                1, 2, 3, 4, 4, 0xaa, 0xbb, 0xaa, 0xbb, 0x10, 0x11, 0x12, 3, 4, 4, 0xaa
            ]
        );
    }

    #[test]
    fn flat_map16_preserves_screen_aliases() {
        let map_pointers = (0..160)
            .map(|screen| {
                let map = screen % 124;
                (map, map)
            })
            .collect();
        let mut importer = Importer {
            constants: Constants::jp(),
            rom: Rom::new(Vec::new()),
            palettes: Vec::new(),
            tiles8: Vec::new(),
            tiles16: Vec::new(),
            tiles32: vec![[1, 2, 3, 4]],
            map_tiles: vec![[[0; 16]; 16]; 160],
            map_pointers,
            map_parents: Vec::new(),
            map_palettes: Vec::new(),
            map_gfx: Vec::new(),
            map_gfx_transition_sheets: Vec::new(),
            sprite_character_rows: HashMap::new(),
            tile_types: Vec::new(),
        };

        let flat = importer.flat_map16(0xb8).unwrap();

        assert_eq!(flat.maps.len(), 124 * 2048);
        assert_eq!(&flat.screen_pointers[..3], &[0x00, 0x80, 0xb8]);
        assert_eq!(&flat.screen_pointers[123 * 3..124 * 3], &[0x00, 0xD8, 0xbf]);
        assert_eq!(&flat.screen_pointers[124 * 3..125 * 3], &[0x00, 0x80, 0xb8]);
    }

    #[test]
    fn tile8_vram_word_round_trips() {
        for word in 0..=u16::MAX {
            assert_eq!(
                Tile8::from_vram_tilemap_word(word).to_vram_tilemap_word(),
                word
            );
        }
    }
}
