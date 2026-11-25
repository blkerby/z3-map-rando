//! Overworld layout in terms of map cell indexes, as used by generation and patching.
//! Saved seeds store it in the room-based form of [`crate::SeedData`].

use std::collections::{BTreeMap, BTreeSet};

use crate::{BorderSide, MapBorder};

/// Placements and themes of the overworld rooms, and the borders between them.
#[derive(Clone, Debug)]
pub struct WorldLayout {
    /// The vanilla layout uses its own connections and edges rather than `borders`.
    pub vanilla: bool,
    /// Every overworld room, at its vanilla position in the vanilla layout.
    pub rooms: Vec<OverworldRoomPlacement>,
    /// Open borders of a randomized layout; any other border is closed.
    pub borders: Vec<MapBorder>,
}

#[derive(Clone, Debug)]
pub struct OverworldRoomPlacement {
    pub room_id: u32,
    pub vanilla_map_id: u8,
    pub position: u8,
    pub size: [u8; 2],
    /// Retiling theme, used in both worlds.
    pub theme: String,
}

impl WorldLayout {
    /// Theme of each themed map area, keyed by vanilla map ID. Overworld rooms theme both
    /// worlds, and special areas take their parent room's theme.
    pub fn build_area_themes(&self) -> BTreeMap<u8, &str> {
        let mut themes = BTreeMap::new();
        for room in &self.rooms {
            themes.insert(room.vanilla_map_id, room.theme.as_str());
            themes.insert(room.vanilla_map_id | 0x40, room.theme.as_str());
        }
        for (_, area, parent_room_id) in common_types::SPECIAL_ROOM_AREAS {
            for room in &self.rooms {
                if room.room_id == parent_room_id {
                    // The first room listed for a shared area sets its theme.
                    themes.entry(area).or_insert(room.theme.as_str());
                }
            }
        }
        themes
    }

    pub fn build_scrollable_transition_pairs(&self) -> BTreeSet<(usize, usize)> {
        if !self.vanilla {
            let mut cells = [0usize; 64];
            for room in &self.rooms {
                for dy in 0..room.size[1] {
                    for dx in 0..room.size[0] {
                        cells[usize::from(room.position + dx + dy * 8)] =
                            usize::from(room.vanilla_map_id);
                    }
                }
            }
            let mut pairs = BTreeSet::new();
            for border in &self.borders {
                let [x, y] = border.position;
                let cell = usize::from(y * 8 + x);
                let a = cells[cell];
                let b = match border.side {
                    BorderSide::East => cells[cell + 1],
                    BorderSide::South => cells[cell + 8],
                };
                pairs.insert((a.min(b), a.max(b)));
                if border.variant != "light-only" && border.variant != "light-and-mirror" {
                    pairs.insert((a.min(b) + 0x40, a.max(b) + 0x40));
                }
            }
            return pairs;
        }
        let light_world_pairs = [
            (0x02, 0x0a),
            (0x03, 0x05),
            (0x05, 0x07),
            (0x0a, 0x12),
            (0x0f, 0x17),
            (0x10, 0x18),
            (0x11, 0x12),
            (0x11, 0x18),
            (0x12, 0x13),
            (0x12, 0x1a),
            (0x13, 0x14),
            (0x14, 0x15),
            (0x15, 0x16),
            (0x15, 0x1d),
            (0x16, 0x17),
            (0x18, 0x22),
            (0x18, 0x29),
            (0x1a, 0x1b),
            (0x1b, 0x25),
            (0x1b, 0x2b),
            (0x1b, 0x2c),
            (0x1d, 0x25),
            (0x1e, 0x2e),
            (0x1e, 0x2f),
            (0x25, 0x2d),
            (0x28, 0x29),
            (0x29, 0x2a),
            (0x2a, 0x32),
            (0x2b, 0x2c),
            (0x2b, 0x33),
            (0x2c, 0x2d),
            (0x2c, 0x34),
            (0x2d, 0x2e),
            (0x2d, 0x35),
            (0x2e, 0x35),
            (0x30, 0x3a),
            (0x32, 0x33),
            (0x33, 0x34),
            (0x33, 0x3b),
            (0x34, 0x3c),
            (0x35, 0x3f),
            (0x37, 0x3f),
            (0x3a, 0x3b),
            (0x3b, 0x3c),
            (0x3c, 0x3f),
        ];
        let mut pairs = BTreeSet::new();
        for (left, right) in light_world_pairs {
            pairs.insert((left, right));
            // The Dark World has no scrolling transition between $7A and $7B.
            if (left, right) != (0x3a, 0x3b) {
                pairs.insert((left + 0x40, right + 0x40));
            }
        }
        pairs
    }

    pub fn get_area_edges(&self, vanilla_map_id: u8) -> Vec<(String, String)> {
        if self.vanilla || vanilla_map_id >= 0x80 {
            return Vec::new();
        }
        let Some(room) = self
            .rooms
            .iter()
            .find(|room| room.vanilla_map_id == vanilla_map_id & 0x3f)
        else {
            return Vec::new();
        };
        let mut edges = Vec::new();
        let x = i16::from(room.position % 8);
        let y = i16::from(room.position / 8);
        let [width, height] = room.size;
        for (side, count) in [
            ("north", width),
            ("south", width),
            ("east", height),
            ("west", height),
        ] {
            for segment in 0..count {
                let segment_offset = i16::from(segment);
                let (cx, cy, nx, ny) = match side {
                    "north" => (x + segment_offset, y, x + segment_offset, y - 1),
                    "south" => (
                        x + segment_offset,
                        y + i16::from(height) - 1,
                        x + segment_offset,
                        y + i16::from(height),
                    ),
                    "east" => (
                        x + i16::from(width) - 1,
                        y + segment_offset,
                        x + i16::from(width),
                        y + segment_offset,
                    ),
                    _ => (x, y + segment_offset, x - 1, y + segment_offset),
                };
                let mut variant = "closed";
                if (0..8).contains(&nx) && (0..8).contains(&ny) {
                    let position = [cx.min(nx) as u8, cy.min(ny) as u8];
                    let border_side = if cx == nx {
                        BorderSide::South
                    } else {
                        BorderSide::East
                    };
                    if let Some(border) = self
                        .borders
                        .iter()
                        .find(|border| border.position == position && border.side == border_side)
                    {
                        variant = &border.variant;
                    }
                }
                let slot = if count == 1 {
                    side.to_owned()
                } else {
                    format!("{side}-{}", segment + 1)
                };
                edges.push((slot, variant.to_owned()));
            }
        }
        edges
    }
}
