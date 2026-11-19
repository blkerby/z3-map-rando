use crate::{BackgroundSettings, CutsceneAction, Palette, PaletteId};
use bincode_next::{Decode, Encode};
use std::collections::{BTreeMap, BTreeSet};

/// Static layers resolved for one saved seed. Runtime state layers remain separate.
#[derive(Clone, Debug, Encode, Decode)]
pub struct ResolvedRetiling {
    pub seed_name: String,
    pub selected: SelectedAreas,
}

#[derive(Clone, Copy, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
pub struct Placement {
    pub palette: PaletteId,
    pub tile: usize,
    pub flip: u8,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ThemeArea {
    pub id: usize,
    pub width: usize,
    pub height: usize,
    pub placements: Vec<Placement>,
    pub map_tiles: Vec<Option<Placement>>,
    pub palettes: BTreeSet<PaletteId>,
    pub extra_tiles: BTreeSet<(PaletteId, usize)>,
    pub overworld_overlay: Option<StateLayer>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Bg1Variant {
    pub name: String,
    pub area: usize,
    pub width: usize,
    pub height: usize,
    pub placements: Vec<Option<Placement>>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct StateLayer {
    pub placements: Vec<Option<Placement>>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ThemeCutscene {
    pub trigger: u8,
    pub area: usize,
    pub actions: Vec<CutsceneAction>,
    pub layers: BTreeMap<String, StateLayer>,
}

impl Placement {
    pub fn canonicalize_flip(&mut self, palettes: &BTreeMap<PaletteId, Palette>) {
        self.flip = palettes[&self.palette].tiles[self.tile].flip_map[usize::from(self.flip)];
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct SelectedAreas {
    pub areas: Vec<ThemeArea>,
    pub background_colors: [u16; 0xa0],
    pub bg1_variants: Vec<Bg1Variant>,
    pub background_settings: BTreeMap<usize, BackgroundSettings>,
    pub cutscenes: Vec<ThemeCutscene>,
}
