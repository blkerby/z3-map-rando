//! Overworld layout in terms of map areas and cell indexes, as used by generation and
//! patching. Saved seeds store it in the room-based form of [`crate::SeedData`].

/// Area themes and, for a randomized layout, room placements and borders.
#[derive(Clone, Debug)]
pub struct WorldLayout {
    pub areas: Vec<AreaLayout>,
    /// Empty for the vanilla layout.
    pub rooms: Vec<OverworldRoomPlacement>,
    pub borders: Vec<CellBorder>,
}

#[derive(Clone, Debug)]
pub struct OverworldRoomPlacement {
    pub room_id: u32,
    pub vanilla_map_id: u8,
    pub position: u8,
    pub size: [u8; 2],
}

/// Cells are indexed as y * 8 + x.
#[derive(Clone, Debug)]
pub struct CellBorder {
    /// Adjacent physical cells, in ascending order; exterior borders are closed.
    pub cells: [u8; 2],
    pub variant: String,
}

impl WorldLayout {
    pub fn get_area_edges(&self, vanilla_map_id: u8) -> Vec<(String, String)> {
        if vanilla_map_id >= 0x80 {
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
                    let first = (cy * 8 + cx) as u8;
                    let second = (ny * 8 + nx) as u8;
                    let cells = [first.min(second), first.max(second)];
                    if let Some(border) = self.borders.iter().find(|border| border.cells == cells) {
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

#[derive(Clone, Debug)]
pub struct AreaLayout {
    /// Top-left vanilla map ID, including the world, not a source room ID or
    /// the area's eventual position.
    pub vanilla_map_id: u8,
    pub theme_name: String,
}
