//! Inventory and route-local state for normal logical traversal.

/// Permanent inventory and progress, held fixed during a traversal wave.
#[derive(Clone, Debug)]
pub struct GlobalState {
    /// Non-progressive item capabilities and prizes, indexed by catalog ItemIndex.
    /// Equipment, Ocarina, resource upgrades, and keys use the fields below.
    /// Maps and Compasses have no logical inventory state.
    pub items: Vec<bool>,
    /// Acquired levels: Sword 0..=4, Shield 0..=3, Glove 0..=2,
    /// Mail 0..=2 (zero is Green Mail), and Boomerang 0..=2.
    pub sword_level: u8,
    pub shield_level: u8,
    pub glove_level: u8,
    pub mail_level: u8,
    pub boomerang_level: u8,
    pub ocarina: OcarinaState,
    pub half_magic: bool,

    /// Health in eighths of a heart.
    pub health_capacity: u8,
    /// Magic in normalized catalog units.
    pub magic_capacity: u8,
    pub bomb_capacity: u8,
    pub arrow_capacity: u8,
    /// Pieces toward the next heart container, 0..=3.
    pub heart_pieces: u8,
    pub bottles_owned: u8,
    pub refills: RefillFlags,

    /// Owned Big Keys, indexed by catalog DungeonIndex.
    pub big_keys: Vec<bool>,
    /// Permanent door access, indexed by catalog DoorIndex, regardless of mechanism.
    /// Small Key grants resolve their stable DoorId and set the same bit as OpenDoor.
    pub unlocked_doors: Vec<bool>,
    /// Permanent progress flags, indexed by catalog FlagIndex.
    pub flags: Vec<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OcarinaState {
    Missing,
    Inactive,
    Active,
}

/// Reusable sources discovered globally, enabling preparation in subsequent waves.
/// Bottle sources never replenish the current route's bottle-use budget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RefillFlags {
    pub health: bool,
    pub magic: bool,
    pub bombs: bool,
    pub arrows: bool,
    pub red_potion: bool,
    pub green_potion: bool,
    pub blue_potion: bool,
    pub fairy: bool,
}

/// Route-local resource usage and temporary state, including alternative followers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalState {
    /// Net health depletion in eighths of a heart; full health has zero used.
    pub health_used: u8,
    pub magic_used: ResourceUsage,
    pub bombs_used: ResourceUsage,
    pub arrows_used: ResourceUsage,
    /// Each use may choose any globally known content. Never reduced along a route.
    pub bottles_used: u8,
    /// Bit (follower as u8) represents a possible catalog Follower.
    /// No follower is always available by freely discarding any follower; zero is valid.
    /// A follower requirement needs its bit present, then narrows the mask to that bit.
    pub possible_followers: u8,
    /// Bit n represents ObstacleIndex(n) in the containing vertex's room.
    /// Room resets set this mask to zero.
    pub cleared_obstacles: u8,
}

/// Net resource depletion, with u8::MAX reserved for an initially-empty resource.
/// Ordinary values increase with spending and decrease with refills.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceUsage(pub u8);

impl ResourceUsage {
    /// No initial preparation: availability stays zero when capacity increases.
    /// A refill replaces this with finite usage relative to the current capacity.
    pub const UNPREPARED: Self = Self(u8::MAX);
}
