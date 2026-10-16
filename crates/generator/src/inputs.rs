use anyhow::{Result, anyhow, bail, ensure};
use common_types::{DoorId, Item};
use hashbrown::HashMap;
use logic_catalog::{DoorIndex, ItemLocationIndex, LockKind, LogicCatalog};
use seed_data::Settings;
use std::collections::BTreeMap;

/// Indexed inputs for traversal and placement, prepared without changing the catalog.
#[derive(Debug)]
pub struct GenerationInputs {
    /// Reward interpretations of catalog definitions; anonymous Small Keys, activation
    /// events such as OcarinaActive, and unsupported receipts are `None`.
    /// Only names used by catalog placements need reward identities here.
    pub items: Vec<Option<Item>>,
    /// Stable authored identities in catalog door order.
    pub door_ids: Vec<DoorId>,
    /// Reverse lookup for catalog doors, independent of Small Key assignments.
    pub door_indices: HashMap<DoorId, DoorIndex>,
    /// Indexed by catalog DungeonIndex, matching `LogicCatalog::dungeons`.
    pub dungeons: Vec<DungeonInputs>,
    /// Eligible ordinary slots, including the slots originally holding Maps and Compasses.
    pub ordinary_locations: Vec<ItemLocationIndex>,
    pub ordinary_pool: Vec<Item>,
    /// Vanilla Big Keys and fairy-exchange rewards. Small Keys are assigned later.
    pub fixed_items: BTreeMap<ItemLocationIndex, Item>,
    pub prize_locations: Vec<ItemLocationIndex>,
    pub prize_pool: Vec<Item>,
    /// Settings resolved by stable tech ID, stored in catalog tech order.
    pub tech_enabled: Vec<bool>,
    pub proficiency_levels: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct DungeonInputs {
    pub small_key_locations: Vec<ItemLocationIndex>,
    pub big_key_locations: Vec<ItemLocationIndex>,
    pub small_key_doors: Vec<DoorIndex>,
    pub map_compass_pool: Vec<Item>,
    pub prize_locations: Vec<ItemLocationIndex>,
    pub ordinary_locations: Vec<ItemLocationIndex>,
}

/// Resolve reward names and player settings, and classify locations by source dungeon.
/// Small Key identities are constructed by the later preliminary assignment pass.
pub fn prepare_inputs(catalog: &LogicCatalog, settings: &Settings) -> Result<GenerationInputs> {
    let mut inputs = GenerationInputs {
        items: Vec::new(),
        door_ids: Vec::new(),
        door_indices: HashMap::new(),
        dungeons: Vec::new(),
        ordinary_locations: Vec::new(),
        ordinary_pool: Vec::new(),
        fixed_items: BTreeMap::new(),
        prize_locations: Vec::new(),
        prize_pool: Vec::new(),
        tech_enabled: vec![false; catalog.techs.len()],
        proficiency_levels: vec![0; catalog.techs.len()],
    };
    for item in &catalog.items {
        inputs.items.push(resolve_item(&item.name));
    }
    for _ in &catalog.dungeons {
        inputs.dungeons.push(DungeonInputs::default());
    }
    for (index, door) in catalog.doors.iter().enumerate() {
        let door_idx = DoorIndex(index as u32);
        // All doors belong to authored rooms; the synthetic flute hub has none.
        let door_id = DoorId {
            room_id: catalog.room(door.room_idx).room_id.unwrap(),
            door_id: door.door_id,
        };
        inputs.door_ids.push(door_id);
        inputs.door_indices.insert(door_id, door_idx);
        if door.lock == LockKind::SmallKey {
            if let Some(dungeon_idx) = catalog.room(door.room_idx).dungeon_idx {
                inputs.dungeons[dungeon_idx.0 as usize]
                    .small_key_doors
                    .push(door_idx);
            }
        }
    }

    let mut ordinary_location_count = 0;
    for (index, location) in catalog.item_locations.iter().enumerate() {
        let location_idx = ItemLocationIndex(index as u32);
        let room_idx = catalog.vertex(location.vertex_idx).room_idx;
        let dungeon = match catalog.room(room_idx).dungeon_idx {
            Some(idx) => Some(&mut inputs.dungeons[idx.0 as usize]),
            None => None,
        };
        let item_name = &catalog.item(location.vanilla_item_idx).name;
        if item_name == "SmallKey" {
            if let Some(dungeon) = dungeon {
                dungeon.small_key_locations.push(location_idx);
            }
            continue;
        }
        let item = inputs.items[location.vanilla_item_idx.0 as usize]
            .ok_or_else(|| anyhow!("unknown reward identity: {item_name}"))?;
        // Waterfall and Pyramid exchanges remain fixed, including progressive equipment rewards.
        if matches!(
            (location.id.room_id, location.id.item_id),
            (common_types::SourceRoomId::Underworld(12 | 69), 1 | 2)
        ) {
            inputs.fixed_items.insert(location_idx, item);
            continue;
        }
        match item {
            Item::BigKey => {
                inputs.fixed_items.insert(location_idx, item);
                if let Some(dungeon) = dungeon {
                    dungeon.big_key_locations.push(location_idx);
                }
            }
            Item::PendantOfCourage
            | Item::PendantOfWisdom
            | Item::PendantOfPower
            | Item::Crystal1
            | Item::Crystal2
            | Item::Crystal3
            | Item::Crystal4
            | Item::Crystal5
            | Item::Crystal6
            | Item::Crystal7 => {
                inputs.prize_locations.push(location_idx);
                inputs.prize_pool.push(item);
                if let Some(dungeon) = dungeon {
                    dungeon.prize_locations.push(location_idx);
                }
            }
            _ => {
                inputs.ordinary_locations.push(location_idx);
                if matches!(item, Item::Map | Item::Compass) {
                    if let Some(dungeon) = dungeon {
                        dungeon.map_compass_pool.push(item);
                        dungeon.ordinary_locations.push(location_idx);
                    }
                } else {
                    ordinary_location_count += 1;
                    if let Some(dungeon) = dungeon {
                        dungeon.ordinary_locations.push(location_idx);
                    }
                }
            }
        }
    }

    for entry in &settings.item_pool {
        match entry.item {
            Item::SmallKey { .. }
            | Item::BigKey
            | Item::Map
            | Item::Compass
            | Item::PendantOfCourage
            | Item::PendantOfWisdom
            | Item::PendantOfPower
            | Item::Crystal1
            | Item::Crystal2
            | Item::Crystal3
            | Item::Crystal4
            | Item::Crystal5
            | Item::Crystal6
            | Item::Crystal7 => bail!(
                "item {:?} is not allowed in the ordinary item pool; dungeon items and prizes use separate placement phases",
                entry.item,
            ),
            _ => {}
        }
        for _ in 0..entry.count {
            inputs.ordinary_pool.push(entry.item);
        }
    }
    // Preserve the prototype's existing pool-size behavior.
    ensure!(
        inputs.ordinary_pool.len() == ordinary_location_count,
        "item pool contains {} items, but {} ordinary locations are available",
        inputs.ordinary_pool.len(),
        ordinary_location_count,
    );

    let mut tech_indices = BTreeMap::new();
    for (index, tech) in catalog.techs.iter().enumerate() {
        tech_indices.insert(tech.tech_id, index);
    }
    for setting in &settings.tech {
        let index = tech_indices
            .get(&setting.tech_id)
            .ok_or_else(|| anyhow!("unknown tech ID: {}", setting.tech_id))?;
        inputs.tech_enabled[*index] = setting.enabled;
    }
    for setting in &settings.proficiencies {
        let index = tech_indices
            .get(&setting.tech_id)
            .ok_or_else(|| anyhow!("unknown proficiency tech ID: {}", setting.tech_id))?;
        inputs.proficiency_levels[*index] = setting.level;
    }
    Ok(inputs)
}

fn resolve_item(name: &str) -> Option<Item> {
    Some(match name {
        "Sword" | "L1Sword" | "L2Sword" | "MasterSword" | "L3Sword" | "L4Sword"
        | "ProgressiveSword" => Item::Sword,
        "Shield" | "BlueShield" | "RedShield" | "MirrorShield" | "ProgressiveShield" => {
            Item::Shield
        }
        "Glove" | "PowerGlove" | "TitansMitt" | "ProgressiveGlove" => Item::Glove,
        "Mail" | "BlueMail" | "RedMail" | "ProgressiveArmor" => Item::Mail,
        "Boomerang" | "BlueBoomerang" | "RedBoomerang" => Item::Boomerang,
        "FireRod" => Item::FireRod,
        "IceRod" => Item::IceRod,
        "Hammer" => Item::Hammer,
        "Hookshot" => Item::Hookshot,
        "Bow" => Item::Bow,
        "MagicPowder" => Item::MagicPowder,
        "Bombos" => Item::Bombos,
        "Ether" => Item::Ether,
        "Quake" => Item::Quake,
        "Lamp" => Item::Lamp,
        "Shovel" => Item::Shovel,
        "CaneOfSomaria" => Item::CaneOfSomaria,
        "CaneOfByrna" => Item::CaneOfByrna,
        "Cape" => Item::Cape,
        "MagicMirror" => Item::MagicMirror,
        "BookOfMudora" => Item::BookOfMudora,
        "Flippers" => Item::Flippers,
        "MoonPearl" => Item::MoonPearl,
        "BugCatchingNet" => Item::BugCatchingNet,
        "Mushroom" => Item::Mushroom,
        "Bottle" => Item::Bottle,
        "PegasusBoots" => Item::PegasusBoots,
        "SilverArrows" | "SilverArrowUpgrade" => Item::SilverArrows,
        "Ocarina" | "OcarinaInactive" => Item::Ocarina,
        "HalfMagic" => Item::HalfMagic,
        "Arrow" => Item::Arrow,
        "Bomb" => Item::Bomb,
        "Heart" => Item::Heart,
        "SmallMagic" => Item::SmallMagic,
        "ThreeBombs" => Item::ThreeBombs,
        "TenArrows" => Item::TenArrows,
        "TenBombs" => Item::TenBombs,
        "Bee" => Item::Bee,
        "Fairy" => Item::Fairy,
        "GoldBee" => Item::GoldBee,
        "RedPotion" => Item::RedPotion,
        "GreenPotion" => Item::GreenPotion,
        "BluePotion" => Item::BluePotion,
        "OneRupee" => Item::OneRupee,
        "FiveRupees" => Item::FiveRupees,
        "TwentyRupees" => Item::TwentyRupees,
        "FiftyRupees" => Item::FiftyRupees,
        "OneHundredRupees" => Item::OneHundredRupees,
        "ThreeHundredRupees" => Item::ThreeHundredRupees,
        "Compass" => Item::Compass,
        "BigKey" => Item::BigKey,
        "Map" => Item::Map,
        "PendantOfCourage" => Item::PendantOfCourage,
        "PendantOfWisdom" => Item::PendantOfWisdom,
        "PendantOfPower" => Item::PendantOfPower,
        "Crystal1" => Item::Crystal1,
        "Crystal2" => Item::Crystal2,
        "Crystal3" => Item::Crystal3,
        "Crystal4" => Item::Crystal4,
        "Crystal5" => Item::Crystal5,
        "Crystal6" => Item::Crystal6,
        "Crystal7" => Item::Crystal7,
        "Triforce" => Item::Triforce,
        "BossHeartContainer" => Item::BossHeartContainer,
        "SanctuaryHeartContainer" => Item::SanctuaryHeartContainer,
        "PieceOfHeart" => Item::PieceOfHeart,
        "BombUpgrade5" => Item::BombUpgrade5,
        "BombUpgrade10" => Item::BombUpgrade10,
        "BombUpgrade50" => Item::BombUpgrade50,
        "ArrowUpgrade5" => Item::ArrowUpgrade5,
        "ArrowUpgrade10" => Item::ArrowUpgrade10,
        "ArrowUpgrade70" => Item::ArrowUpgrade70,
        _ => return None,
    })
}
