//! ROM assembly shared by native and future browser callers.

use crate::{
    Patcher, PcAddr, SeedData, SnesAddr, TransitionAssetPhase, asset_bundle,
    import::{FlatMap16, Importer},
    retiling as theme, verify_vanilla_rom,
};
use anyhow::{Context, Result, ensure};
use common_types::{Item, SourceRoomId};
use patch_catalog::PatchCatalog;
use retiling_catalog::RetilingCatalog;
use seed_data::{
    AreaLayout, CustomizationSettings, OverworldRoomPlacement, StartingBoomerang, StartingBottle,
    StartingBow, StartingOcarina, WorldLayout,
};
use std::collections::BTreeMap;

// Temporary address space used only to split the importer's vanilla flat maps.
const VANILLA_FLAT_MAPS_START: SnesAddr = SnesAddr(0xb80000);
const BANK_SIZE: usize = 0x8000;

/// Choices applied while patching, independently of saved generation settings.
#[derive(Clone, Debug)]
pub struct PatchSettings {
    pub transition_asset_phase: TransitionAssetPhase,
    pub customization: CustomizationSettings,
}

struct FlatMapTables {
    flat: FlatMap16,
    bg1_pointers: Vec<u8>,
    area_80_bg1_pointers: Vec<u8>,
    lost_woods_clear_bg1_pointers: Vec<u8>,
    rain_pointers: Vec<u8>,
}

/// Create a ROM from saved choices using the current catalogs.
pub fn patch_rom(
    rom: &[u8],
    seed: &SeedData,
    retiling: &RetilingCatalog,
    patches: &PatchCatalog,
    settings: &PatchSettings,
) -> Result<Vec<u8>> {
    if let Some(resolved) = &retiling.resolved {
        ensure!(
            resolved.seed_name == seed.metadata.seed_name,
            "retiling catalog is for seed {}, expected {}",
            resolved.seed_name,
            seed.metadata.seed_name
        );
    }
    verify_vanilla_rom(rom)?;
    let mut output = rom.to_vec();
    let layout = resolve_world_layout(seed, patches)?;
    let mut patcher = prepare_retiling(
        &mut output,
        &layout,
        retiling,
        patches,
        settings,
        seed.metadata.rng_seed.parse()?,
    )?;
    crate::items::apply_items(&mut patcher, rom, &seed.rooms, patches)?;
    let starting = &seed.settings.starting_state;
    let equipment = &starting.equipment;
    let resources = &starting.resources;
    let counts = equipment.count_items();
    let mut inventory = vec![0_u8; 0x51];
    inventory[0x39] = 0xf8; // Native lift, read, talk, and pull ability flags.
    for (item, offset, value) in [
        (Item::Hookshot, 0x02, 1),
        (Item::FireRod, 0x05, 1),
        (Item::IceRod, 0x06, 1),
        (Item::Bombos, 0x07, 1),
        (Item::Ether, 0x08, 1),
        (Item::Quake, 0x09, 1),
        (Item::Lamp, 0x0a, 1),
        (Item::Hammer, 0x0b, 1),
        (Item::BugCatchingNet, 0x0d, 1),
        (Item::BookOfMudora, 0x0e, 1),
        (Item::CaneOfSomaria, 0x10, 1),
        (Item::CaneOfByrna, 0x11, 1),
        (Item::Cape, 0x12, 1),
        (Item::MagicMirror, 0x13, 2),
        (Item::PegasusBoots, 0x15, 1),
        (Item::Flippers, 0x16, 1),
        (Item::MoonPearl, 0x17, 1),
        (Item::HalfMagic, 0x3b, 1),
    ] {
        if counts[&item] > 0 {
            inventory[offset] = value;
        }
    }
    for (item, offset) in [
        (Item::Sword, 0x19),
        (Item::Shield, 0x1a),
        (Item::Glove, 0x14),
        (Item::Mail, 0x1b),
        (Item::Boomerang, 0x01),
    ] {
        inventory[offset] = counts[&item] as u8;
    }
    inventory[0] = match equipment.bow {
        StartingBow::None => 0,
        StartingBow::Bow => {
            if resources.arrows > 0 {
                2
            } else {
                1
            }
        }
        StartingBow::Silver => {
            if resources.arrows > 0 {
                4
            } else {
                3
            }
        }
    };
    // Permanent ownership for ALTTPR-compatible autotrackers, independent of ammunition.
    inventory[0x4e] = match equipment.bow {
        StartingBow::None => 0,
        StartingBow::Bow => 0x80,
        StartingBow::Silver => 0xc0,
    };
    match equipment.boomerang {
        StartingBoomerang::None => {}
        StartingBoomerang::Blue => inventory[0x4c] |= 0x80,
        StartingBoomerang::Red => inventory[0x4c] |= 0xc0,
    }
    if equipment.mushroom {
        inventory[0x04] = 1;
        inventory[0x4c] |= 0x28;
    }
    if equipment.magic_powder {
        inventory[0x04] = 2;
        inventory[0x4c] |= 0x10;
    }
    if equipment.shovel {
        inventory[0x0c] = 1;
        inventory[0x4c] |= 0x04;
    }
    match equipment.ocarina {
        StartingOcarina::None => {}
        StartingOcarina::Inactive => {
            inventory[0x0c] = 2;
            inventory[0x4c] |= 0x02;
        }
        StartingOcarina::Active => {
            inventory[0x0c] = 3;
            inventory[0x4c] |= 0x01;
        }
    }
    let bottles = [
        resources.bottle_1,
        resources.bottle_2,
        resources.bottle_3,
        resources.bottle_4,
    ];
    for (i, bottle) in bottles.into_iter().enumerate() {
        inventory[0x1c + i] = match bottle {
            StartingBottle::None => 0,
            StartingBottle::Empty => 2,
            StartingBottle::RedPotion => 3,
            StartingBottle::GreenPotion => 4,
            StartingBottle::BluePotion => 5,
            StartingBottle::Fairy => 6,
            StartingBottle::Bee => 7,
            StartingBottle::GoldBee => 8,
        };
        // Select the first owned bottle.
        if inventory[0x0f] == 0 && bottle != StartingBottle::None {
            inventory[0x0f] = i as u8 + 1;
        }
    }
    if equipment.pegasus_boots {
        inventory[0x39] |= 0x04;
    }
    if equipment.flippers {
        inventory[0x39] |= 0x02;
    }
    inventory[0x03] = resources.bombs;
    inventory[0x37] = resources.arrows;
    inventory[0x30] = resources.bomb_capacity;
    inventory[0x31] = resources.arrow_capacity;
    inventory[0x2c] = resources.heart_capacity * 8;
    inventory[0x2d] = inventory[0x2c];
    inventory[0x20..0x22].copy_from_slice(&resources.rupees.to_le_bytes());
    inventory[0x22..0x24].copy_from_slice(&resources.rupees.to_le_bytes());
    inventory[0x50] = 2; // Game state: Zelda rescued.
    patcher
        .context("starting state")
        .write(SnesAddr(patches.symbols.starting_state).into(), inventory)?;
    patcher.context("capacity upgrade sizes").write(
        SnesAddr(patches.symbols.capacity_upgrade_sizes).into(),
        vec![
            seed.settings.item_pool.bomb_upgrade_size,
            seed.settings.item_pool.arrow_upgrade_size,
        ],
    )?;
    let rgb = u32::from_str_radix(&settings.customization.heart_color, 16)
        .context("parsing heart color")?;
    let bgr555 = (rgb >> 19 & 0x1f) | (rgb >> 11 & 0x1f) << 5 | (rgb >> 3 & 0x1f) << 10;
    patcher.context("heart color").write(
        SnesAddr(patches.symbols.heart_color).into(),
        (bgr555 as u16).to_le_bytes().to_vec(),
    )?;
    patcher.apply(&mut output)?;
    output[0x7fd7] = 0x0c;
    finalize_rom(&mut output);
    Ok(output)
}

/// Express the seed's room choices as map areas and cell indexes.
pub(crate) fn resolve_world_layout(seed: &SeedData, patches: &PatchCatalog) -> Result<WorldLayout> {
    let mut themes = BTreeMap::new();
    let mut rooms = Vec::new();
    for room in &seed.rooms {
        let Some(area) = patches.room_areas.get(&room.room_id) else {
            continue;
        };
        if let Some(theme) = &room.theme {
            let mut area_ids = vec![area.area_id];
            if let SourceRoomId::Overworld(_) = room.room_id {
                area_ids.push(area.area_id | 0x40);
            }
            for area_id in area_ids {
                if let Some(existing) = themes.insert(area_id, theme) {
                    // Sacred Grove and Under the Bridge share an area.
                    ensure!(
                        existing == theme,
                        "rooms sharing area ${area_id:02X} have different themes"
                    );
                }
            }
        }
        if let (SourceRoomId::Overworld(room_id), Some([x, y])) = (room.room_id, room.position) {
            rooms.push(OverworldRoomPlacement {
                room_id,
                vanilla_map_id: area.area_id,
                position: y * 8 + x,
                size: area.size,
            });
        }
    }
    let mut areas = Vec::new();
    for (vanilla_map_id, theme) in themes {
        areas.push(AreaLayout {
            vanilla_map_id,
            theme_name: theme.clone(),
        });
    }
    Ok(WorldLayout {
        areas,
        rooms,
        borders: seed.borders.clone(),
    })
}

/// Apply the FastROM base needed for import, then prepare the remaining writes.
fn prepare_retiling(
    rom: &mut Vec<u8>,
    layout: &WorldLayout,
    catalog: &RetilingCatalog,
    patches: &PatchCatalog,
    settings: &PatchSettings,
    palette_seed: u64,
) -> Result<Patcher> {
    let mut fastrom_base = Patcher::default();
    fastrom_base.use_ips(&patches.patches.fastrom_base)?;
    fastrom_base.apply(rom)?;

    let mut importer = Importer::new(rom.clone())?;
    let vanilla_flat = importer.flat_map16((VANILLA_FLAT_MAPS_START.0 >> 16) as u8)?;
    let vanilla_maps = split_flat_maps(&vanilla_flat)?;
    let tile_types = importer.tile_types()?.to_owned();
    let vanilla_tiles = importer.tiles16()?.to_owned();
    let area_assets = importer.overworld_area_assets()?;
    let credits_cool_background = importer.credits_cool_background_assets()?;
    let sprite_seed = importer.overworld_sprite_seed()?;

    let compiled = theme::compile(
        catalog,
        &importer,
        &vanilla_tiles,
        &tile_types,
        area_assets,
        layout,
        palette_seed,
    )?;
    let credits_overworld = importer.credits_overworld_assets(&compiled.area_assets)?;
    let flat = assemble_flat_maps(
        vanilla_maps,
        &compiled.screen_maps,
        &compiled.bg1_variants,
        &compiled.rain_maps,
        patches.symbols.theme_flat_map16_data,
    )?;
    let bundle = asset_bundle::build(
        &compiled.area_assets,
        &credits_overworld,
        &credits_cool_background,
        &sprite_seed,
        &compiled.dynamic_tile_groups,
        &compiled.cutscenes,
        &compiled.overworld_overlays,
        settings.transition_asset_phase,
        &patches.symbols,
    )?;

    rom.resize(4 * 1024 * 1024, 0);
    let mut patcher = Patcher::default();
    patcher.context("flat Map16 data").write(
        SnesAddr(patches.symbols.theme_flat_map16_data).into(),
        flat.flat.maps,
    )?;
    patcher.context("flat Map16 pointers").write(
        SnesAddr(patches.symbols.flat_map16_screen_pointers).into(),
        flat.flat.screen_pointers,
    )?;
    patcher.context("BG1 flat Map16 pointers").write(
        SnesAddr(patches.symbols.bg1_flat_map16_screen_pointers).into(),
        flat.bg1_pointers,
    )?;
    patcher.context("area $80 BG1 variant pointers").write(
        SnesAddr(patches.symbols.area_80_bg1_flat_map16_pointers).into(),
        flat.area_80_bg1_pointers,
    )?;
    patcher.context("generated BG1 enable marker").write(
        SnesAddr(patches.symbols.generated_bg1_enabled).into(),
        vec![1],
    )?;
    patcher
        .context("Lost Woods clear BG1 variant pointers")
        .write(
            SnesAddr(patches.symbols.lost_woods_clear_bg1_flat_map16_pointers).into(),
            flat.lost_woods_clear_bg1_pointers,
        )?;
    patcher.context("rain contexts").write(
        SnesAddr(patches.symbols.rain_contexts).into(),
        compiled.rain_contexts.to_vec(),
    )?;
    patcher.context("rain map pointers").write(
        SnesAddr(patches.symbols.rain_flat_map16_pointers).into(),
        flat.rain_pointers,
    )?;
    let mut context = patcher.context("authored world maps");
    context.write(
        SnesAddr(patches.symbols.world_map_graphics).into(),
        compiled.world_map.graphics,
    )?;
    context.write(
        SnesAddr(patches.symbols.world_map_tilemaps).into(),
        compiled.world_map.tilemaps,
    )?;
    context.write(
        SnesAddr(patches.symbols.world_map_palettes).into(),
        compiled.world_map.palettes,
    )?;

    let mut context = patcher.context("expanded Map16 definitions");
    for (start, definitions) in [
        patches.symbols.map16_top_left,
        patches.symbols.map16_top_right,
        patches.symbols.map16_bottom_left,
        patches.symbols.map16_bottom_right,
    ]
    .into_iter()
    .zip(compiled.map16_definitions)
    {
        context.write(SnesAddr(start).into(), definitions)?;
    }
    let mut context = patcher.context("Map16 quadrant properties");
    for (start, properties) in [
        patches.symbols.map16_property_top_left,
        patches.symbols.map16_property_top_right,
        patches.symbols.map16_property_bottom_left,
        patches.symbols.map16_property_bottom_right,
    ]
    .into_iter()
    .zip(compiled.map16_properties)
    {
        context.write(SnesAddr(start).into(), properties)?;
    }
    let mut background_colors = Vec::with_capacity(compiled.background_colors.len() * 2);
    for color in compiled.background_colors {
        background_colors.extend_from_slice(&color.to_le_bytes());
    }
    patcher.context("theme background colors").write(
        SnesAddr(patches.symbols.theme_background_colors).into(),
        background_colors,
    )?;
    patcher.context("overworld asset pointer table").write(
        SnesAddr(bundle.pointer_table_start).into(),
        bundle.pointer_table,
    )?;
    patcher
        .context("overworld asset data")
        .write(SnesAddr(bundle.data_start).into(), bundle.data)?;

    for patch in [
        &patches.patches.fastrom_extra,
        &patches.patches.rom_size,
        &patches.patches.new_game,
        &patches.patches.messages,
        &patches.patches.world_state,
        &patches.patches.mothula_sword,
        &patches.patches.overworld_map_data,
        &patches.patches.overworld_map16_graphics,
        &patches.patches.overworld_map16_properties,
        &patches.patches.bg3_tilemap,
        &patches.patches.heart_color,
        &patches.patches.overworld_bg_tilemaps,
        &patches.patches.overworld_camera_alignment,
        &patches.patches.nmi_optimize,
        &patches.patches.mirror_bg1_hdma,
        &patches.patches.overworld_vram,
        &patches.patches.overworld_assets,
        &patches.patches.overworld_animations,
        &patches.patches.overworld_entrances,
        &patches.patches.overworld_bg_color,
        &patches.patches.overworld_dynamic_tiles,
        &patches.patches.overworld_cutscenes,
        &patches.patches.overworld_lightning,
        &patches.patches.overworld_music,
        &patches.patches.overworld_topology,
        &patches.patches.overworld_world_map,
    ] {
        patcher.use_ips(patch)?;
    }
    if settings.customization.reduced_flashing {
        patcher.use_ips(&patches.patches.reduced_flashing)?;
    }
    crate::topology::apply_topology(&mut patcher, rom, layout, patches)?;
    Ok(patcher)
}

/// Update the checksum and complement after assembling the complete 4 MiB ROM.
fn finalize_rom(rom: &mut [u8]) {
    rom[0x7fdc..0x7fe0].copy_from_slice(&[0xff, 0xff, 0, 0]);
    let mut checksum = 0u16;
    for &byte in rom.iter() {
        checksum = checksum.wrapping_add(u16::from(byte));
    }
    rom[0x7fdc..0x7fde].copy_from_slice(&(!checksum).to_le_bytes());
    rom[0x7fde..0x7fe0].copy_from_slice(&checksum.to_le_bytes());
}

fn get_flat_map(flat: &FlatMap16, screen: usize) -> Result<Vec<u8>> {
    let pointer = &flat.screen_pointers[screen * 3..screen * 3 + 3];
    let pointer = u32::from_le_bytes([pointer[0], pointer[1], pointer[2], 0]);
    let map_start: PcAddr = SnesAddr(pointer).into();
    let flat_start: PcAddr = VANILLA_FLAT_MAPS_START.into();
    let offset = usize::try_from(map_start.0 - flat_start.0)?;
    Ok(flat.maps[offset..offset + 0x800].to_vec())
}

fn split_flat_maps(flat: &FlatMap16) -> Result<Vec<Vec<u8>>> {
    let mut maps = Vec::with_capacity(0xa0);
    for screen in 0..0xa0 {
        maps.push(get_flat_map(flat, screen)?);
    }
    Ok(maps)
}

fn assemble_flat_maps(
    mut maps: Vec<Vec<u8>>,
    replacements: &BTreeMap<usize, Vec<u8>>,
    bg1_variants: &BTreeMap<usize, Vec<theme::CompiledBg1Variant>>,
    rain_maps: &[Vec<u8>; 2],
    map_data_start: u32,
) -> Result<FlatMapTables> {
    for (screen, map) in replacements {
        ensure!(
            *screen < maps.len(),
            "theme screen {screen:02X} is out of range"
        );
        maps[*screen] = map.clone();
    }

    let mut indices = BTreeMap::new();
    let mut data = Vec::new();
    let mut pointers = Vec::with_capacity(0xa0 * 3);
    for map in &maps {
        let address = intern_flat_map(map, &mut indices, &mut data, map_data_start)?;
        pointers.extend_from_slice(&address.to_le_bytes()[..3]);
    }

    let mut bg1_pointers = vec![0; 0xa0 * 3];
    let mut area_80_pointers = [0; 2];
    let mut lost_woods_clear_pointers = [0; 10];
    for (&area, variants) in bg1_variants {
        for variant in variants {
            let lost_woods_clear = area == 0x00 && variant.name == "Woods Clear";
            let area_80_index = if area == 0x80 {
                Some(match variant.name.as_str() {
                    "Grove Fog" => 0,
                    "Bridge Shadow" => 1,
                    name => anyhow::bail!("unknown area $80 BG1 variant {name}"),
                })
            } else {
                None
            };
            for (&screen, map) in &variant.maps {
                ensure!(screen < 0xa0, "BG1 screen {screen:02X} is out of range");
                ensure!(
                    replacements.contains_key(&screen),
                    "BG1 screen {screen:02X} has no generated BG2 map"
                );
                let address = intern_flat_map(map, &mut indices, &mut data, map_data_start)?;
                if (area != 0x80 || area_80_index == Some(0)) && !lost_woods_clear {
                    let offset = screen * 3;
                    ensure!(
                        bg1_pointers[offset..offset + 3] == [0; 3],
                        "multiple BG1 maps target screen {screen:02X}"
                    );
                    bg1_pointers[offset..offset + 3].copy_from_slice(&address.to_le_bytes()[..3]);
                }
                if lost_woods_clear {
                    lost_woods_clear_pointers[screen] = address;
                }
                if let Some(index) = area_80_index {
                    ensure!(
                        screen == 0x80,
                        "area $80 BG1 has unexpected map {screen:02X}"
                    );
                    area_80_pointers[index] = address;
                }
            }
        }
    }
    let mut rain_pointers = Vec::with_capacity(6);
    for map in rain_maps {
        let address = intern_flat_map(map, &mut indices, &mut data, map_data_start)?;
        rain_pointers.extend_from_slice(&address.to_le_bytes()[..3]);
    }
    ensure!(
        data.len() <= 10 * BANK_SIZE,
        "flat maps exceed reserved banks C0-C9"
    );
    let mut area_80_bg1_pointers = Vec::with_capacity(6);
    for pointer in area_80_pointers {
        area_80_bg1_pointers.extend_from_slice(&pointer.to_le_bytes()[..3]);
    }
    let mut lost_woods_clear_bg1_pointers = Vec::with_capacity(30);
    for pointer in lost_woods_clear_pointers {
        lost_woods_clear_bg1_pointers.extend_from_slice(&pointer.to_le_bytes()[..3]);
    }
    Ok(FlatMapTables {
        flat: FlatMap16 {
            maps: data,
            screen_pointers: pointers,
        },
        bg1_pointers,
        area_80_bg1_pointers,
        lost_woods_clear_bg1_pointers,
        rain_pointers,
    })
}

fn intern_flat_map(
    map: &[u8],
    indices: &mut BTreeMap<Vec<u8>, usize>,
    data: &mut Vec<u8>,
    map_data_start: u32,
) -> Result<u32> {
    ensure!(map.len() == 0x800);
    let index = if let Some(&index) = indices.get(map) {
        index
    } else {
        let index = indices.len();
        indices.insert(map.to_vec(), index);
        data.extend_from_slice(map);
        index
    };
    Ok(map_data_start + u32::try_from(index / 16)? * 0x10000 + u32::try_from(index % 16)? * 0x0800)
}
