use anyhow::{Result, anyhow, bail, ensure};
use common_types::{DoorId, Item, SourceRoomId};
use hashbrown::HashMap;
use logic_catalog::{DoorIndex, ItemLocationIndex, LockKind, LogicCatalog};
use seed_data::Settings;
use std::collections::BTreeMap;

/// Indexed inputs for traversal and placement, prepared without changing the catalog.
#[derive(Debug)]
pub struct GenerationInputs {
    /// Item interpretations of catalog definitions; anonymous Small Keys, activation
    /// events such as OcarinaActive, and unsupported receipts are `None`.
    /// Only names used by catalog placements need item identities here.
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
    /// Vanilla Big Keys and fairy-exchange items. Small Keys are assigned later.
    pub fixed_items: BTreeMap<ItemLocationIndex, Item>,
    pub prize_locations: Vec<ItemLocationIndex>,
    pub prize_pool: Vec<Item>,
    /// Settings resolved by stable tech ID, stored in catalog tech order.
    pub tech_enabled: Vec<bool>,
    /// Settings resolved by room identity and notable ID, in catalog notable order.
    pub notable_enabled: Vec<bool>,
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

/// Resolve item names and player settings, and classify locations by source dungeon.
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
        notable_enabled: vec![false; catalog.notables.len()],
        proficiency_levels: vec![0; catalog.techs.len()],
    };
    for item in &catalog.items {
        inputs.items.push(item.item);
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
            .ok_or_else(|| anyhow!("unknown item identity: {item_name}"))?;
        // Waterfall and Pyramid exchanges remain fixed, including progressive equipment items.
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

    for (&item, &count) in &settings.item_pool {
        match item {
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
                item,
            ),
            _ => {}
        }
        for _ in 0..count {
            inputs.ordinary_pool.push(item);
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
    for (tech_id, &enabled) in &settings.skill.tech {
        let index = tech_indices
            .get(tech_id)
            .ok_or_else(|| anyhow!("unknown tech ID: {tech_id}"))?;
        inputs.tech_enabled[*index] = enabled;
    }
    let mut notable_indices = HashMap::new();
    for (index, notable) in catalog.notables.iter().enumerate() {
        let room_id = catalog.room(notable.room_idx).room_id.unwrap();
        notable_indices.insert((room_id, notable.notable_id), index);
    }
    let notables = &settings.skill.notables;
    let mut notable_settings = Vec::new();
    for (&room, room_notables) in &notables.overworld {
        notable_settings.push((SourceRoomId::Overworld(room), room_notables));
    }
    for (&room, room_notables) in &notables.underworld {
        notable_settings.push((SourceRoomId::Underworld(room), room_notables));
    }
    for (room_id, room_notables) in notable_settings {
        for (&notable_id, &enabled) in room_notables {
            let index = notable_indices
                .get(&(room_id, notable_id))
                .ok_or_else(|| anyhow!("unknown notable ID {notable_id} in room {room_id:?}"))?;
            inputs.notable_enabled[*index] = enabled;
        }
    }
    for (tech_id, &level) in &settings.skill.proficiencies {
        let index = tech_indices
            .get(tech_id)
            .ok_or_else(|| anyhow!("unknown proficiency tech ID: {tech_id}"))?;
        inputs.proficiency_levels[*index] = level;
    }
    Ok(inputs)
}
