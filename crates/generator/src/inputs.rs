use anyhow::{Result, anyhow, bail};
use common_types::{DoorId, FAIRY_UPGRADE_LOCATIONS, Item, ItemKind, SourceRoomId};
use hashbrown::HashMap;
use logic_catalog::{DoorIndex, DungeonIndex, ItemLocationIndex, LockKind, LogicCatalog};
use seed_data::{Settings, SkillSettings, StartingState};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct InvalidSettings(pub String);

impl std::fmt::Display for InvalidSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidSettings {}

/// Indexed inputs for traversal and placement, prepared without changing the catalog.
#[derive(Debug)]
pub struct GenerationInputs {
    /// Item kinds of catalog definitions; activation events such as OcarinaActive and
    /// unsupported receipts are `None`.
    pub items: Vec<Option<ItemKind>>,
    /// Stable authored identities in catalog door order.
    pub door_ids: Vec<DoorId>,
    /// Reverse lookup for catalog doors, independent of Small Key assignments.
    pub door_indices: HashMap<DoorId, DoorIndex>,
    /// Indexed by catalog DungeonIndex, matching `LogicCatalog::dungeons`.
    pub dungeons: Vec<DungeonInputs>,
    /// Reverse lookup of dungeons by the room ID that dungeon items carry.
    pub dungeon_indices: HashMap<SourceRoomId, DungeonIndex>,
    /// Eligible ordinary slots, including the slots originally holding Maps and Compasses.
    pub ordinary_locations: Vec<ItemLocationIndex>,
    /// Great Fairy upgrade locations, filled from the ordinary pool before other locations.
    pub fairy_locations: Vec<ItemLocationIndex>,
    /// Ordinary pickups, padded with Nothing to fill the available locations.
    pub ordinary_pool: Vec<Item>,
    /// Vanilla Big Keys. Small Keys are assigned later.
    pub fixed_items: BTreeMap<ItemLocationIndex, Item>,
    pub prize_locations: Vec<ItemLocationIndex>,
    pub prize_pool: Vec<Item>,
    pub skill: SkillInputs,
    pub starting_state: StartingState,
    pub bomb_upgrade_size: u8,
    pub arrow_upgrade_size: u8,
}

/// Skill settings resolved to catalog order.
#[derive(Debug)]
pub struct SkillInputs {
    /// Settings resolved by stable tech ID, stored in catalog tech order.
    pub tech_enabled: Vec<bool>,
    pub can_convert_to_pearlless_link: bool,
    /// Settings resolved by room identity and notable ID, in catalog notable order.
    pub notable_enabled: Vec<bool>,
    pub proficiency_levels: Vec<u32>,
}

#[derive(Debug)]
pub struct DungeonInputs {
    /// The dungeon's identity in its Map, Compass, and Big Key items.
    pub room_id: SourceRoomId,
    pub small_key_locations: Vec<ItemLocationIndex>,
    pub big_key_locations: Vec<ItemLocationIndex>,
    pub small_key_doors: Vec<DoorIndex>,
    pub map_compass_pool: Vec<Item>,
    pub prize_locations: Vec<ItemLocationIndex>,
    pub ordinary_locations: Vec<ItemLocationIndex>,
}

/// Resolve skill, starting state, and catalog door/dungeon lookups for spoiler replay without
/// validating or building the generation item pool.
pub fn prepare_spoiler_inputs(
    catalog: &LogicCatalog,
    settings: &Settings,
) -> Result<GenerationInputs> {
    let mut inputs = GenerationInputs {
        items: Vec::new(),
        door_ids: Vec::new(),
        door_indices: HashMap::new(),
        dungeons: Vec::new(),
        dungeon_indices: HashMap::new(),
        ordinary_locations: Vec::new(),
        fairy_locations: Vec::new(),
        ordinary_pool: Vec::new(),
        fixed_items: BTreeMap::new(),
        prize_locations: Vec::new(),
        prize_pool: Vec::new(),
        skill: prepare_skill_inputs(catalog, &settings.skill)?,
        starting_state: settings.starting_state.clone(),
        bomb_upgrade_size: settings.item_pool.bomb_upgrade_size,
        arrow_upgrade_size: settings.item_pool.arrow_upgrade_size,
    };
    for (index, &room_idx) in catalog.dungeons.iter().enumerate() {
        let room_id = catalog.room(room_idx).room_id.unwrap();
        inputs
            .dungeon_indices
            .insert(room_id, DungeonIndex(index as u32));
    }
    for (index, door) in catalog.doors.iter().enumerate() {
        let door_idx = DoorIndex(index as u32);
        // All doors belong to authored rooms; the synthetic flute hub has none.
        let door_id = DoorId {
            room_id: catalog.room(door.room_idx).room_id.unwrap(),
            door_id: door.door_id,
        };
        inputs.door_indices.insert(door_id, door_idx);
    }
    Ok(inputs)
}

/// Resolve item names and player settings, and classify locations by source dungeon.
/// Small Key identities are constructed by the later preliminary assignment pass.
pub fn prepare_inputs(catalog: &LogicCatalog, settings: &Settings) -> Result<GenerationInputs> {
    let mut inputs = prepare_spoiler_inputs(catalog, settings)?;
    for item in &catalog.items {
        inputs.items.push(item.item);
    }
    for &room_idx in &catalog.dungeons {
        let room_id = catalog.room(room_idx).room_id.unwrap();
        inputs.dungeons.push(DungeonInputs {
            room_id,
            small_key_locations: Vec::new(),
            big_key_locations: Vec::new(),
            small_key_doors: Vec::new(),
            map_compass_pool: Vec::new(),
            prize_locations: Vec::new(),
            ordinary_locations: Vec::new(),
        });
    }
    for (index, door) in catalog.doors.iter().enumerate() {
        let door_idx = DoorIndex(index as u32);
        let door_id = DoorId {
            room_id: catalog.room(door.room_idx).room_id.unwrap(),
            door_id: door.door_id,
        };
        inputs.door_ids.push(door_id);
        if door.lock == LockKind::SmallKey {
            if let Some(dungeon_idx) = catalog.room(door.room_idx).dungeon_idx {
                inputs.dungeons[dungeon_idx.0 as usize]
                    .small_key_doors
                    .push(door_idx);
            }
        }
    }

    let ordinary_location_count = catalog.count_ordinary_item_locations();
    for (index, location) in catalog.item_locations.iter().enumerate() {
        let location_idx = ItemLocationIndex(index as u32);
        let room_idx = catalog.vertex(location.vertex_idx).room_idx;
        let dungeon = match catalog.room(room_idx).dungeon_idx {
            Some(idx) => Some(&mut inputs.dungeons[idx.0 as usize]),
            None => None,
        };
        let item_name = &catalog.item(location.vanilla_item_idx).name;
        let kind = inputs.items[location.vanilla_item_idx.0 as usize]
            .ok_or_else(|| anyhow!("unknown item identity: {item_name}"))?;
        if kind == ItemKind::SmallKey {
            if let Some(dungeon) = dungeon {
                dungeon.small_key_locations.push(location_idx);
            }
            continue;
        }
        if FAIRY_UPGRADE_LOCATIONS.contains(&location.id) {
            inputs.fairy_locations.push(location_idx);
            continue;
        }
        match kind {
            ItemKind::BigKey => {
                // Vanilla dungeon items are always inside their dungeon.
                let dungeon = dungeon.unwrap();
                inputs
                    .fixed_items
                    .insert(location_idx, Item::BigKey(dungeon.room_id));
                dungeon.big_key_locations.push(location_idx);
            }
            ItemKind::PendantOfCourage
            | ItemKind::PendantOfWisdom
            | ItemKind::PendantOfPower
            | ItemKind::Crystal1
            | ItemKind::Crystal2
            | ItemKind::Crystal3
            | ItemKind::Crystal4
            | ItemKind::Crystal5
            | ItemKind::Crystal6
            | ItemKind::Crystal7 => {
                inputs.prize_locations.push(location_idx);
                inputs.prize_pool.push(Item::from_kind(kind).unwrap());
                if let Some(dungeon) = dungeon {
                    dungeon.prize_locations.push(location_idx);
                }
            }
            ItemKind::Map | ItemKind::Compass => {
                inputs.ordinary_locations.push(location_idx);
                let dungeon = dungeon.unwrap();
                dungeon.map_compass_pool.push(if kind == ItemKind::Map {
                    Item::Map(dungeon.room_id)
                } else {
                    Item::Compass(dungeon.room_id)
                });
                dungeon.ordinary_locations.push(location_idx);
            }
            _ => {
                inputs.ordinary_locations.push(location_idx);
                if let Some(dungeon) = dungeon {
                    dungeon.ordinary_locations.push(location_idx);
                }
            }
        }
    }

    let item_pool = settings
        .resolve_item_pool()
        .map_err(|error| InvalidSettings(error.to_string()))?;
    let mut pool_count = 0_u64;
    for &count in item_pool.values() {
        pool_count += u64::from(count);
    }
    if pool_count > u64::from(ordinary_location_count) {
        return Err(InvalidSettings(format!(
            "item pool contains {pool_count} items, but only {ordinary_location_count} ordinary locations are available"
        )).into());
    }
    for (&kind, &count) in &item_pool {
        let item = match kind {
            ItemKind::SmallKey
            | ItemKind::BigKey
            | ItemKind::Map
            | ItemKind::Compass
            | ItemKind::PendantOfCourage
            | ItemKind::PendantOfWisdom
            | ItemKind::PendantOfPower
            | ItemKind::Crystal1
            | ItemKind::Crystal2
            | ItemKind::Crystal3
            | ItemKind::Crystal4
            | ItemKind::Crystal5
            | ItemKind::Crystal6
            | ItemKind::Crystal7 => bail!(
                "item {:?} is not allowed in the ordinary item pool; dungeon items and prizes use separate placement phases",
                kind,
            ),
            _ => Item::from_kind(kind).unwrap(),
        };
        for _ in 0..count {
            inputs.ordinary_pool.push(item);
        }
    }
    for _ in pool_count..u64::from(ordinary_location_count) {
        inputs.ordinary_pool.push(Item::Nothing);
    }

    Ok(inputs)
}

/// Resolve skill settings by stable tech, room, and notable IDs.
pub fn prepare_skill_inputs(
    catalog: &LogicCatalog,
    skill_settings: &SkillSettings,
) -> Result<SkillInputs> {
    let mut skill = SkillInputs {
        tech_enabled: vec![false; catalog.techs.len()],
        can_convert_to_pearlless_link: false,
        notable_enabled: vec![false; catalog.notables.len()],
        proficiency_levels: vec![0; catalog.techs.len()],
    };
    let mut tech_indices = BTreeMap::new();
    for (index, tech) in catalog.techs.iter().enumerate() {
        tech_indices.insert(tech.tech_id, index);
    }
    for (tech_id, &enabled) in &skill_settings.tech {
        let index = tech_indices
            .get(tech_id)
            .ok_or_else(|| anyhow!("unknown tech ID: {tech_id}"))?;
        skill.tech_enabled[*index] = enabled;
        if enabled
            && matches!(
                catalog.techs[*index].name.as_str(),
                "canCarryLonk" | "canPearllessLink"
            )
        {
            skill.can_convert_to_pearlless_link = true;
        }
    }
    let mut notable_indices = HashMap::new();
    for (index, notable) in catalog.notables.iter().enumerate() {
        let room_id = catalog.room(notable.room_idx).room_id.unwrap();
        notable_indices.insert((room_id, notable.notable_id), index);
    }
    for (&room_id, room_notables) in &skill_settings.notables {
        for (&notable_id, &enabled) in room_notables {
            let index = notable_indices
                .get(&(room_id, notable_id))
                .ok_or_else(|| anyhow!("unknown notable ID {notable_id} in room {room_id:?}"))?;
            skill.notable_enabled[*index] = enabled;
        }
    }
    for (tech_id, &level) in &skill_settings.proficiencies {
        let index = tech_indices
            .get(tech_id)
            .ok_or_else(|| anyhow!("unknown proficiency tech ID: {tech_id}"))?;
        skill.proficiency_levels[*index] = level;
    }
    Ok(skill)
}
