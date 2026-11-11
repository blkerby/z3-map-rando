use crate::{Patcher, PcAddr, SnesAddr, import::Importer};
use anyhow::Result;
use common_types::Item;
use patch_catalog::PatchCatalog;
use seed_data::ItemPlacement;

/// Add saved item writes using the current catalog's encodings and carrier hooks.
pub(crate) fn apply_items(
    patcher: &mut Patcher,
    rom: &[u8],
    placements: &[ItemPlacement],
    catalog: &PatchCatalog,
) -> Result<()> {
    patcher.use_ips(&catalog.patches.item_inventory)?;
    patcher.use_ips(&catalog.patches.item_menu_toggle)?;
    patcher.use_ips(&catalog.patches.item_placements)?;
    let tables = catalog.symbols.item_receipt_tables;
    let bank = tables & 0xFF0000;
    let mut context = patcher.context("vanilla item receipt tables");
    for (source, destination, length) in [
        (0x898366, tables, 0x4C),
        (0x8983B2, tables + 0x100, 0x4C),
        (0x8983FE, tables + 0x200, 0x4C),
        (0x89844A, tables + 0x300, 0x4C),
        (0x898496, tables + 0x400, 0x4C),
        (0x8984E2, tables + 0x500, 0x98),
        (0x89857A, tables + 0x700, 0x4C),
        (0x88C301, tables + 0x800, 0x98),
        (0x8985C6, bank | 0x85C6, 0x1C),
        (0x89891E, bank | 0x891E, 0x0C),
    ] {
        let start = PcAddr::from(SnesAddr(source)).0 as usize;
        let mut bytes = rom[start..start + length].to_vec();
        if destination == tables + 0x400 {
            bytes[1] = 0x05; // Use normal sword OAM properties for the Master Sword.
        }
        context.write(SnesAddr(destination).into(), bytes)?;
    }
    let graphics = Importer::new(rom.to_vec())?.read_item_graphics()?;
    patcher
        .context("vanilla item graphics")
        .write(SnesAddr(catalog.symbols.item_graphics).into(), graphics)?;
    for placement in placements {
        let location = &catalog.item_locations[&placement.item_location_id];
        if location.rom_addresses.is_empty() {
            continue;
        }
        let item_name = match placement.item {
            Item::Sword => "ProgressiveSword",
            Item::Shield => "ProgressiveShield",
            Item::Glove => "ProgressiveGlove",
            Item::Mail => "ProgressiveArmor",
            Item::Boomerang => "ProgressiveBoomerang",
            Item::FireRod => "FireRod",
            Item::IceRod => "IceRod",
            Item::Hammer => "Hammer",
            Item::Hookshot => "Hookshot",
            Item::Bow => "Bow",
            Item::SilverArrows => "BowAndSilverArrows",
            Item::MagicPowder => "MagicPowder",
            Item::Bombos => "Bombos",
            Item::Ether => "Ether",
            Item::Quake => "Quake",
            Item::Lamp => "Lamp",
            Item::Shovel => "Shovel",
            Item::Ocarina => "OcarinaInactive",
            Item::CaneOfSomaria => "CaneOfSomaria",
            Item::CaneOfByrna => "CaneOfByrna",
            Item::Cape => "Cape",
            Item::MagicMirror => "MagicMirror",
            Item::BookOfMudora => "BookOfMudora",
            Item::Flippers => "Flippers",
            Item::MoonPearl => "MoonPearl",
            Item::BugCatchingNet => "BugCatchingNet",
            Item::Mushroom => "Mushroom",
            Item::PegasusBoots => "PegasusBoots",
            Item::Bottle => "Bottle",
            Item::Bee => "Bee",
            Item::Fairy => "Fairy",
            Item::GoldBee => "GoldBee",
            Item::RedPotion => "RedPotion",
            Item::GreenPotion => "GreenPotion",
            Item::BluePotion => "BluePotion",
            Item::BossHeartContainer => "BossHeartContainer",
            Item::SanctuaryHeartContainer => "SanctuaryHeartContainer",
            Item::PieceOfHeart => "PieceOfHeart",
            Item::HalfMagic => "HalfMagic",
            Item::BombUpgrade5 => "BombUpgrade5",
            Item::BombUpgrade10 => "BombUpgrade10",
            Item::BombUpgrade50 => "BombUpgrade50",
            Item::ArrowUpgrade5 => "ArrowUpgrade5",
            Item::ArrowUpgrade10 => "ArrowUpgrade10",
            Item::ArrowUpgrade70 => "ArrowUpgrade70",
            Item::Arrow => "OneArrow",
            Item::TenArrows => "TenArrows",
            Item::Bomb => "OneBomb",
            Item::ThreeBombs => "ThreeBombs",
            Item::TenBombs => "TenBombs",
            Item::Heart => "Heart",
            Item::SmallMagic => "SmallMagic",
            Item::OneRupee => "OneRupee",
            Item::FiveRupees => "FiveRupees",
            Item::TwentyRupees => "TwentyRupees",
            Item::FiftyRupees => "FiftyRupees",
            Item::OneHundredRupees => "OneHundredRupees",
            Item::ThreeHundredRupees => "ThreeHundredRupees",
            Item::Map => "Map",
            Item::Compass => "Compass",
            Item::SmallKey { .. } => "SmallKey",
            Item::BigKey => "BigKey",
            Item::PendantOfCourage => "PendantOfCourage",
            Item::PendantOfWisdom => "PendantOfWisdom",
            Item::PendantOfPower => "PendantOfPower",
            Item::Crystal1 => "Crystal1",
            Item::Crystal2 => "Crystal2",
            Item::Crystal3 => "Crystal3",
            Item::Crystal4 => "Crystal4",
            Item::Crystal5 => "Crystal5",
            Item::Crystal6 => "Crystal6",
            Item::Crystal7 => "Crystal7",
            Item::Triforce => "Triforce",
        };
        let item = &catalog.items[item_name];
        let mut context = patcher.context(&format!("item {:?}", placement.item_location_id));
        if let Some(bytes) = &item.prize_patch_bytes {
            for (&address, &value) in location.rom_addresses.iter().zip(bytes) {
                context.write(PcAddr(address), vec![value])?;
            }
        } else {
            context.write(PcAddr(location.rom_addresses[0]), vec![item.receipt_id])?;
        }
    }
    Ok(())
}
