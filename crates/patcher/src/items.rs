use crate::{Patcher, PcAddr};
use anyhow::Result;
use common_types::Item;
use patch_catalog::PatchCatalog;
use seed_data::ItemPlacement;

/// Add saved item writes using the current catalog's encodings and carrier hooks.
pub(crate) fn apply_items(
    patcher: &mut Patcher,
    placements: &[ItemPlacement],
    catalog: &PatchCatalog,
) -> Result<()> {
    patcher.use_ips(&catalog.patches.item_placements)?;
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
