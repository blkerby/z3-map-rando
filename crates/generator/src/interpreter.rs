//! Strat evaluation over a fixed global state and evolving local states.

use crate::{
    graph::TraversalEdge,
    inputs::GenerationInputs,
    state::{GlobalState, LocalState, RefillFlags, ResourceUsage},
};
use hashbrown::HashSet;
use logic_catalog::{
    Ammo, BottleContent, DoorIndex, Effect, Equipment, FlagIndex, ItemLocationIndex, LockKind,
    LogicCatalog, OcarinaState, PrizeKind, Requirement, Resource, UniqueItem,
};

/// Newly reachable item locations and permanent progress relative to the input snapshot.
/// The normal traversal's closure loop applies them between waves;
/// the global state is not updated within a wave itself. Reached requirement
/// discoveries are retained even when a later requirement fails.
#[derive(Debug, Default)]
pub struct GlobalDiscoveries {
    /// Newly reachable collection actions, including unfilled locations.
    /// Applying their rewards is tracked separately by GlobalState::collected_item_locations.
    pub item_locations: HashSet<ItemLocationIndex>,
    pub flags: HashSet<FlagIndex>,
    pub unlocked_doors: HashSet<DoorIndex>,
    pub activate_ocarina: bool,
    pub refills: RefillFlags,
}

#[derive(Debug, Default)]
pub struct EdgeResult {
    pub local_states: Vec<LocalState>,
    pub discoveries: GlobalDiscoveries,
}

pub struct Interpreter<'a> {
    pub catalog: &'a LogicCatalog,
    pub inputs: &'a GenerationInputs,
    pub global: &'a GlobalState,
}

impl Interpreter<'_> {
    /// Report global discoveries as a requirement prefix succeeds, even if a later part fails.
    /// Returned local states and strat effects require the whole requirement to succeed.
    pub fn evaluate_edge(&self, edge: &TraversalEdge, local: &LocalState) -> EdgeResult {
        let mut result = EdgeResult::default();
        let states = self.evaluate_requirement(
            &edge.edge.requirement,
            local.clone(),
            &mut result.discoveries,
        );
        for mut state in states {
            for effect in &edge.edge.effects {
                match *effect {
                    Effect::CollectItem(idx) => {
                        if !self.global.reachable_item_locations[idx.0 as usize] {
                            result.discoveries.item_locations.insert(idx);
                        }
                    }
                    Effect::ActivateOcarina => {
                        result.discoveries.activate_ocarina =
                            self.global.ocarina != OcarinaState::Active;
                    }
                    Effect::SetFlag(idx) => {
                        if !self.global.flags[idx.0 as usize] {
                            result.discoveries.flags.insert(idx);
                        }
                    }
                    Effect::OpenDoor(idx) => {
                        if !self.global.unlocked_doors[idx.0 as usize] {
                            result.discoveries.unlocked_doors.insert(idx);
                        }
                    }
                    Effect::ClearObstacle(idx) => state.cleared_obstacles |= 1 << idx.0,
                    Effect::ResetObstacle(idx) => state.cleared_obstacles &= !(1 << idx.0),
                    Effect::SetFollower(follower) => {
                        state.possible_followers.0 = match follower {
                            Some(follower) => 1 << follower as u8,
                            None => 0,
                        };
                    }
                }
            }
            if !result.local_states.contains(&state) {
                result.local_states.push(state);
            }
        }
        result
    }

    fn evaluate_requirement(
        &self,
        requirement: &Requirement,
        mut state: LocalState,
        discoveries: &mut GlobalDiscoveries,
    ) -> Vec<LocalState> {
        let succeeds = match requirement {
            Requirement::Free | Requirement::Pay(_) => true,
            Requirement::Never => false,
            Requirement::And(children) => {
                let mut states = vec![state];
                for child in children {
                    let mut next = Vec::new();
                    for state in states {
                        for output in self.evaluate_requirement(child, state, discoveries) {
                            push_distinct_state(&mut next, output);
                        }
                    }
                    states = next;
                    if states.is_empty() {
                        break;
                    }
                }
                return states;
            }
            Requirement::Or(children) => {
                let mut states = Vec::new();
                for child in children {
                    for output in self.evaluate_requirement(child, state.clone(), discoveries) {
                        push_distinct_state(&mut states, output);
                    }
                }
                return states;
            }
            Requirement::Item(item) => self.global.unique_items[*item as usize],
            Requirement::Ocarina(minimum) => self.global.ocarina >= *minimum,
            Requirement::Bottles(minimum) => self.global.bottles_owned >= *minimum,
            Requirement::BigKey(idx) => self.global.big_keys[idx.0 as usize],
            Requirement::Equipment {
                equipment,
                minimum_level,
            } => {
                let level = match equipment {
                    Equipment::Sword => self.global.sword_level,
                    Equipment::Shield => self.global.shield_level,
                    Equipment::Glove => self.global.glove_level,
                    Equipment::Boomerang => self.global.boomerang_level,
                };
                level >= *minimum_level
            }
            Requirement::Tech(idx) => self.inputs.tech_enabled[idx.0 as usize],
            Requirement::Proficiency { tech_idx, minimum } => {
                self.inputs.proficiency_levels[tech_idx.0 as usize] >= *minimum
            }
            Requirement::Flag(idx) => self.global.flags[idx.0 as usize],
            Requirement::PrizeCount { kind, minimum } => {
                let items: &[UniqueItem] = match kind {
                    PrizeKind::Pendant => &[
                        UniqueItem::PendantOfCourage,
                        UniqueItem::PendantOfWisdom,
                        UniqueItem::PendantOfPower,
                    ],
                    PrizeKind::Crystal => &[
                        UniqueItem::Crystal1,
                        UniqueItem::Crystal2,
                        UniqueItem::Crystal3,
                        UniqueItem::Crystal4,
                        UniqueItem::Crystal5,
                        UniqueItem::Crystal6,
                        UniqueItem::Crystal7,
                    ],
                };
                let mut count = 0;
                for &item in items {
                    if self.global.unique_items[item as usize] {
                        count += 1;
                    }
                }
                count >= *minimum
            }
            Requirement::UseAmmo { kind, count } => {
                let (usage, capacity) = match kind {
                    Ammo::Arrow | Ammo::SilverArrow => {
                        if !self.global.unique_items[UniqueItem::Bow as usize]
                            || (*kind == Ammo::SilverArrow
                                && !self.global.unique_items[UniqueItem::SilverArrows as usize])
                        {
                            return Vec::new();
                        }
                        (&mut state.arrows_used, self.global.arrow_capacity)
                    }
                    Ammo::Bomb => (&mut state.bombs_used, self.global.bomb_capacity),
                };
                if *count == 0 {
                    true
                } else if *usage == ResourceUsage::UNPREPARED
                    || *count > u32::from(capacity.saturating_sub(usage.0))
                {
                    false
                } else {
                    usage.0 += *count as u8;
                    true
                }
            }
            Requirement::UseMagic {
                cost_per_use,
                num_uses,
            } => {
                let half_magic = self.global.unique_items[UniqueItem::HalfMagic as usize];
                let divisor = if half_magic { 2 } else { 1 };
                let cost_per_use = u64::from(*cost_per_use);
                let total_cost = (cost_per_use * u64::from(*num_uses)).div_ceil(divisor);
                if total_cost > 0 && state.magic_used == ResourceUsage::UNPREPARED {
                    return Vec::new();
                }
                let mut states = vec![state];
                let mut completed_cost = 0;
                while !states.is_empty() {
                    // Batch all affordable uses up to the next required potion.
                    let usage = states[0].magic_used;
                    let available = if usage == ResourceUsage::UNPREPARED {
                        0
                    } else {
                        u64::from(self.global.magic_capacity.saturating_sub(usage.0))
                    };
                    let remaining_cost = total_cost - completed_cost;
                    if remaining_cost <= available {
                        if remaining_cost > 0 {
                            for state in &mut states {
                                state.magic_used.0 += remaining_cost as u8;
                            }
                        }
                        return states;
                    }

                    // Invert the rounded cumulative cost to find the last affordable use.
                    let completed_uses = (completed_cost + available) * divisor / cost_per_use;
                    let affordable_cost = (cost_per_use * completed_uses).div_ceil(divisor);
                    let next_cost = (cost_per_use * (completed_uses + 1)).div_ceil(divisor);
                    let cost = next_cost - affordable_cost;
                    if cost > u64::from(self.global.magic_capacity) {
                        return Vec::new();
                    }
                    completed_cost = next_cost;
                    let mut next = Vec::new();
                    for state in states {
                        for content in [BottleContent::BluePotion, BottleContent::GreenPotion] {
                            if let Some(mut refilled) = self.use_potion(&state, content) {
                                refilled.magic_used = ResourceUsage(cost as u8);
                                push_distinct_state(&mut next, refilled);
                                break;
                            }
                        }
                    }
                    states = next;
                }
                return states;
            }
            Requirement::Damage { per_mail, hits } => {
                let damage = per_mail[self.global.mail_level as usize];
                let mut states = vec![state];
                for _ in 0..*hits {
                    let mut next = Vec::new();
                    for mut state in states {
                        let remaining = self.global.health_capacity - state.health_used;
                        if damage < u32::from(remaining) {
                            state.health_used += damage as u8;
                            push_distinct_state(&mut next, state);
                        } else {
                            if damage < u32::from(self.global.health_capacity) {
                                for content in [BottleContent::BluePotion, BottleContent::RedPotion]
                                {
                                    if let Some(mut healed) = self.use_potion(&state, content) {
                                        healed.health_used = damage as u8;
                                        push_distinct_state(&mut next, healed);
                                        break;
                                    }
                                }
                            }
                            if self.global.refills.fairy
                                && state.bottles_used < self.global.bottles_owned
                            {
                                state.bottles_used += 1;
                                state.health_used = self.global.health_capacity
                                    - self.global.health_capacity.min(56);
                                push_distinct_state(&mut next, state);
                            }
                        }
                    }
                    states = next;
                    if states.is_empty() {
                        break;
                    }
                }
                return states;
            }
            Requirement::Refill { resource, limit } => {
                if *limit == 0 {
                    return vec![state];
                }
                match resource {
                    Resource::Health => {
                        let target_used = self.global.health_capacity
                            - (*limit).min(u32::from(self.global.health_capacity)) as u8;
                        state.health_used = state.health_used.min(target_used);
                    }
                    Resource::Magic => {
                        refill_resource(&mut state.magic_used, self.global.magic_capacity, *limit);
                        discoveries.refills.magic = !self.global.refills.magic;
                    }
                    Resource::Bombs => {
                        refill_resource(&mut state.bombs_used, self.global.bomb_capacity, *limit);
                        discoveries.refills.bombs = !self.global.refills.bombs;
                    }
                    Resource::Arrows => {
                        refill_resource(&mut state.arrows_used, self.global.arrow_capacity, *limit);
                        discoveries.refills.arrows = !self.global.refills.arrows;
                    }
                    Resource::Rupees => {}
                    Resource::BottleContent(BottleContent::Bee | BottleContent::GoldBee) => {}
                    Resource::BottleContent(BottleContent::Fairy) => {
                        let healing = u64::from(*limit) * 56;
                        let target_used = self.global.health_capacity
                            - healing.min(u64::from(self.global.health_capacity)) as u8;
                        state.health_used = state.health_used.min(target_used);
                        if self.global.bottles_owned > 0
                            && self.global.unique_items[UniqueItem::BugCatchingNet as usize]
                        {
                            discoveries.refills.fairy = !self.global.refills.fairy;
                        }
                    }
                    Resource::BottleContent(content) => {
                        if self.global.bottles_owned == 0 {
                            return Vec::new();
                        }
                        match content {
                            BottleContent::RedPotion => {
                                discoveries.refills.red_potion = !self.global.refills.red_potion;
                            }
                            BottleContent::GreenPotion => {
                                discoveries.refills.green_potion =
                                    !self.global.refills.green_potion;
                            }
                            BottleContent::BluePotion => {
                                discoveries.refills.blue_potion = !self.global.refills.blue_potion;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                true
            }
            Requirement::ResourceMissingAtMost { resource, count } => {
                let missing = match resource {
                    Resource::Health => u32::from(state.health_used),
                    Resource::Magic => {
                        if state.magic_used == ResourceUsage::UNPREPARED {
                            u32::from(self.global.magic_capacity)
                        } else {
                            u32::from(state.magic_used.0)
                        }
                    }
                    Resource::Bombs => {
                        if state.bombs_used == ResourceUsage::UNPREPARED {
                            u32::from(self.global.bomb_capacity)
                        } else {
                            u32::from(state.bombs_used.0)
                        }
                    }
                    Resource::Arrows => {
                        if state.arrows_used == ResourceUsage::UNPREPARED {
                            u32::from(self.global.arrow_capacity)
                        } else {
                            u32::from(state.arrows_used.0)
                        }
                    }
                    Resource::Rupees => 0,
                    Resource::BottleContent(BottleContent::Bee | BottleContent::GoldBee) => 0,
                    Resource::BottleContent(content) => {
                        if self.have_bottle_content(*content) {
                            u32::from(state.bottles_used)
                        } else {
                            u32::from(self.global.bottles_owned)
                        }
                    }
                };
                if missing <= *count {
                    return vec![state];
                }
                if *resource == Resource::Magic && state.magic_used == ResourceUsage::UNPREPARED {
                    return Vec::new();
                }
                let contents: &[BottleContent] = match resource {
                    Resource::Health => &[BottleContent::BluePotion, BottleContent::RedPotion],
                    Resource::Magic => &[BottleContent::BluePotion, BottleContent::GreenPotion],
                    _ => return Vec::new(),
                };
                let mut states = Vec::new();
                for &content in contents {
                    if let Some(refilled) = self.use_potion(&state, content) {
                        push_distinct_state(&mut states, refilled);
                    }
                }
                return states;
            }
            Requirement::Follower(follower) => {
                let bit = 1 << *follower as u8;
                if state.possible_followers.0 & bit == 0 {
                    false
                } else {
                    state.possible_followers.0 = bit;
                    true
                }
            }
            Requirement::LoseFollowers(mask) => {
                state.possible_followers.0 &= !mask.0;
                true
            }
            Requirement::ResetObstacles(_) => {
                state.cleared_obstacles = 0;
                true
            }
            Requirement::ResetHealthToSpawn => {
                state.health_used = self.global.health_capacity - self.global.get_spawn_health();
                true
            }
            Requirement::ObstacleCleared(idx) => state.cleared_obstacles & (1 << idx.0) != 0,
            Requirement::ObstacleNotCleared(idx) => state.cleared_obstacles & (1 << idx.0) == 0,
            Requirement::Door(idx) => {
                if self.global.unlocked_doors[idx.0 as usize] {
                    true
                } else {
                    let door = self.catalog.door(*idx);
                    if door.lock == LockKind::BigKey
                        && self.global.big_keys
                            [self.catalog.room(door.room_idx).dungeon_idx.unwrap().0 as usize]
                    {
                        discoveries.unlocked_doors.insert(*idx);
                        true
                    } else {
                        false
                    }
                }
            }
        };
        if succeeds { vec![state] } else { Vec::new() }
    }

    /// Called only when a health or magic requirement cannot succeed without a refill.
    fn use_potion(&self, state: &LocalState, content: BottleContent) -> Option<LocalState> {
        if state.bottles_used >= self.global.bottles_owned || !self.have_bottle_content(content) {
            return None;
        }
        let mut refilled = state.clone();
        match content {
            BottleContent::BluePotion => {
                refilled.health_used = 0;
                refill_resource(
                    &mut refilled.magic_used,
                    self.global.magic_capacity,
                    u32::from(self.global.magic_capacity),
                );
            }
            BottleContent::RedPotion => refilled.health_used = 0,
            BottleContent::GreenPotion => refill_resource(
                &mut refilled.magic_used,
                self.global.magic_capacity,
                u32::from(self.global.magic_capacity),
            ),
            _ => return None,
        }
        refilled.bottles_used += 1;
        Some(refilled)
    }

    fn have_bottle_content(&self, content: BottleContent) -> bool {
        match content {
            BottleContent::RedPotion => self.global.refills.red_potion,
            BottleContent::GreenPotion => self.global.refills.green_potion,
            BottleContent::BluePotion => self.global.refills.blue_potion,
            BottleContent::Fairy => self.global.refills.fairy,
            BottleContent::Bee | BottleContent::GoldBee => false,
        }
    }
}

fn push_distinct_state(states: &mut Vec<LocalState>, state: LocalState) {
    if !states.contains(&state) {
        states.push(state);
    }
}

fn refill_resource(usage: &mut ResourceUsage, capacity: u8, limit: u32) {
    if *usage == ResourceUsage::UNPREPARED {
        return;
    }
    let target_used = capacity - limit.min(u32::from(capacity)) as u8;
    usage.0 = usage.0.min(target_used);
}
