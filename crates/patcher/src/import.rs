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
    tile_types: Vec<u8>,
}

pub struct FlatMap16 {
    pub maps: Vec<u8>,
    pub screen_pointers: Vec<u8>,
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

    pub fn flat_map16(&mut self) -> Result<FlatMap16> {
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

            let pointer =
                (u32::from(0xa0 + index / 16) << 16) | (0x8000 + u32::from(index % 16) * 0x0800);
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

        for i in 0..113 {
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
            ensure!(data.len() == 0x600, "unexpected graphics sheet length");

            for j in 0..64 {
                let mut tile = [[0; 8]; 8];
                for (y, row) in tile.iter_mut().enumerate() {
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let c0 = (data[j * 24 + y * 2] >> (7 - x)) & 1;
                        let c1 = (data[j * 24 + y * 2 + 1] >> (7 - x)) & 1;
                        let c2 = (data[j * 24 + y + 16] >> (7 - x)) & 1;
                        *pixel = c0 | c1 << 1 | c2 << 2;
                    }
                }
                self.tiles8.push(tile);
            }
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
                0
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
            let mut gfx: Vec<u8> = self
                .rom
                .read_n((self.constants.global_gfx_set_addr + global * 8).into(), 8)?
                .to_owned();
            if let Some(addr) = self.constants.custom_gfx_set_addr {
                let local = self.rom.read_n((addr + u32::from(parent) * 8).into(), 8)?;
                for i in 0..8 {
                    if local[i] != 0xff {
                        gfx[i] = local[i];
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
                    }
                }
            }
            self.map_gfx.push(gfx.try_into().unwrap());
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
            tile_types: Vec::new(),
        };

        let flat = importer.flat_map16().unwrap();

        assert_eq!(flat.maps.len(), 124 * 2048);
        assert_eq!(&flat.screen_pointers[..3], &[0x00, 0x80, 0xa0]);
        assert_eq!(&flat.screen_pointers[123 * 3..124 * 3], &[0x00, 0xD8, 0xa7]);
        assert_eq!(&flat.screen_pointers[124 * 3..125 * 3], &[0x00, 0x80, 0xa0]);
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
