use super::is_overworld_overlay;
use crate::retiling::condition::evaluate_condition;
use anyhow::Result;
use retiling_catalog::{Background, CutsceneAction, EdgeConditions, RetilingCatalog};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceRoom {
    name: String,
    dark_world_name: String,
    position: [u8; 2],
    edge_variants: BTreeMap<String, Vec<SourceSegment>>,
}

#[derive(Deserialize)]
struct SourceSegment {
    supported: BTreeMap<String, serde_json::Value>,
}

struct EdgeSlot {
    name: String,
    side: usize,
    segment: usize,
    variants: Vec<String>,
}

struct CollisionLayer<'a> {
    condition: &'a str,
    writes: Vec<(usize, u8)>,
}

struct CollisionArea<'a> {
    width: usize,
    height: usize,
    layers: Vec<CollisionLayer<'a>>,
}

fn classify_collision(collision: u8) -> u8 {
    match collision {
        0x00 | 0x48 | 0x4a => 0x00,
        0x01 | 0x02 | 0x03 | 0x26 | 0x43 => 0x01,
        _ => collision,
    }
}

fn prepare_collision(catalog: &RetilingCatalog, area_id: u8) -> CollisionArea<'_> {
    let area = catalog
        .areas
        .values()
        .find(|area| area.vanilla_map_id == Some(area_id))
        .unwrap();
    let theme = &area.themes["desert"];
    let width = usize::from(theme.layers[0].grid.width);
    let height = usize::from(theme.layers[0].grid.height);
    let mut cutscene_layers = BTreeSet::new();
    for cutscene in &theme.cutscenes {
        for action in &cutscene.actions {
            if let CutsceneAction::Draw { layer } = action {
                cutscene_layers.insert(layer.as_str());
            }
        }
    }
    let mut layers = Vec::new();
    for layer in &theme.layers {
        if layer.background != Background::Bg2
            || is_overworld_overlay(usize::from(area_id), layer)
            || cutscene_layers.contains(layer.name.as_str())
        {
            continue;
        }
        let mut writes = Vec::with_capacity(layer.grid.tiles.len());
        for placement in &layer.grid.tiles {
            let mut property =
                catalog.palettes[&placement.palette].tiles[usize::from(placement.tile)].collision;
            if (0x10..0x1c).contains(&property) {
                property ^= placement.flip as u8;
            }
            writes.push((
                usize::from(placement.y) * width + usize::from(placement.x),
                classify_collision(property),
            ));
        }
        layers.push(CollisionLayer {
            condition: layer
                .condition
                .as_deref()
                .filter(|condition| !condition.trim().is_empty())
                .unwrap_or("true"),
            writes,
        });
    }
    CollisionArea {
        width,
        height,
        layers,
    }
}

fn compose_collision(
    area: &CollisionArea<'_>,
    conditions: &EdgeConditions,
    edges: &[(String, String)],
) -> Result<Vec<u8>> {
    let mut collision = vec![0xff; area.width * area.height];
    for layer in &area.layers {
        if !evaluate_condition(
            layer.condition,
            edges,
            &conditions.edge_variants,
            &conditions.edge_variant_aliases,
            &conditions.slot_aliases,
        )? {
            continue;
        }
        for &(offset, property) in &layer.writes {
            collision[offset] = property;
        }
    }
    Ok(collision)
}

// Builds a fresh Desert catalog from the source submodules without a ROM and
// enumerates every supported edge-variant combination in both worlds.
// Collision writes are prepared once per area; each combination still evaluates
// every layer condition and applies selected layers in their authored order.
//
// Checks four tiles of inward clearance for each opening from get_edge_openings, with
// a solid tile or correctly oriented slope at either end of the edge strip. Water
// variants' water openings must be deep water rather than ground, and specialized
// variants' openings may also end at a ledge or shallow water, where land meets water.
// Openings of light-only and light-and-mirror are checked only in the Light World.
// Inner and outer slopes must have their solid corner outside the aperture; inward from
// the edge strip, a specialized variant's opening may narrow diagonally at slopes or
// ledges. The end of a lake's land opening toward the water is unchecked.
// Shallow water counts as ground or deep water in the three strips inward from the edge strip.
// Collision classes combine ordinary/diggable ground and full-tile blockers,
// preserving slopes and direction.
//
// Distinct combinations must produce distinct paired-world collision layers;
// light-only and light-and-mirror count as one geometry choice.
// Uniqueness failures report one pair per room with the fewest differing slots.
// Aperture failures report the failing strip closest to the transition across
// all combinations, with an example combination.
//
// Run: cargo test -p patcher --release check_overworld_edge_geometry -- --nocapture
#[test]
fn check_overworld_edge_geometry() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temporary = tempfile::tempdir()?;
    let catalog_path = temporary.path().join("retiling.bin");
    catalog_builder::retiling::build_catalog(
        &root.join("ALTTPRetiling"),
        &catalog_path,
        &root.join("data/tile_fingerprints.json"),
        1,
    )?;
    let catalog = retiling_catalog::decode_catalog(fs::File::open(catalog_path)?)?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(root.join("z3-json-data/rooms/overworld"))? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }
    paths.sort();
    let mut failures = BTreeMap::<String, Vec<(String, String)>>::new();
    let mut aperture_failures = BTreeMap::new();
    let mut configurations = 0;
    let mut apertures = 0;
    for path in paths {
        let room: SourceRoom = serde_json::from_slice(&fs::read(path)?)?;
        let area_id = room.position[1] * 8 + room.position[0];
        let mut slots = Vec::new();
        for (side, name) in ["north", "south", "east", "west"].into_iter().enumerate() {
            let segments = &room.edge_variants[name];
            for (segment, source) in segments.iter().enumerate() {
                let name = if segments.len() == 1 {
                    name.to_owned()
                } else {
                    format!("{name}-{}", segment + 1)
                };
                let mut variants = Vec::new();
                for variant in source.supported.keys() {
                    variants.push(variant.clone());
                }
                slots.push(EdgeSlot {
                    name,
                    side,
                    segment,
                    variants,
                });
            }
        }
        let collision_areas = [
            prepare_collision(&catalog, area_id),
            prepare_collision(&catalog, area_id + 0x40),
        ];
        let mut selection = vec![0; slots.len()];
        let mut seen = BTreeMap::<[u8; 32], Vec<Vec<(String, String)>>>::new();
        let mut duplicate: Option<(usize, [Vec<(String, String)>; 2])> = None;
        'combinations: loop {
            let mut edges = Vec::new();
            for (slot, &index) in slots.iter().zip(&selection) {
                let variant = &slot.variants[index];
                edges.push((slot.name.clone(), variant.clone()));
            }
            let mut combined = Vec::new();
            for (world, area) in [0, 0x40].into_iter().zip(&collision_areas) {
                let area_name = if world == 0 {
                    &room.name
                } else {
                    &room.dark_world_name
                };
                let collision = compose_collision(area, &catalog.conditions, &edges)?;
                let (width, height) = (area.width, area.height);
                for (slot, (_, variant)) in slots.iter().zip(&edges) {
                    // These openings exist only in the Light World.
                    if world == 0x40
                        && matches!(variant.as_str(), "light-only" | "light-and-mirror")
                    {
                        continue;
                    }
                    let openings = super::get_edge_openings(slot.side < 2, variant);
                    // Which openings are deep water, in get_edge_openings order; the rest are ground.
                    let water_openings: &[bool] = match variant.as_str() {
                        "horizonal-water-land" | "vertical-water-land" => &[true, false],
                        "horizontal-land-water" | "lake" => &[false, true],
                        "land-water-land" => &[false, true, false],
                        _ => &[],
                    };
                    let specialized = !matches!(
                        variant.as_str(),
                        "small-1" | "small-2" | "dual-small" | "dual-upper" | "wide"
                    );
                    let boundary_slopes = match slot.side {
                        0 => [0x10, 0x11], // Northwest, northeast.
                        1 => [0x12, 0x13], // Southwest, southeast.
                        2 => [0x11, 0x13], // Northeast, southeast.
                        _ => [0x10, 0x12], // Northwest, southwest.
                    };
                    let mut strips = [[0; 64]; 4];
                    for (depth, strip) in strips.iter_mut().enumerate() {
                        for (along, property) in strip.iter_mut().enumerate() {
                            let offset = slot.segment * 64 + along;
                            let (x, y) = match slot.side {
                                0 => (offset, depth),
                                1 => (offset, height - 1 - depth),
                                2 => (width - 1 - depth, offset),
                                _ => (depth, offset),
                            };
                            *property = collision[y * width + x];
                        }
                    }
                    for (opening, &(start, end)) in openings.iter().enumerate() {
                        apertures += 1;
                        let fill = if water_openings.get(opening) == Some(&true) {
                            0x08
                        } else {
                            0x00
                        };
                        let mut expected = vec![fill; end - start + 2];
                        expected[0] = 0x01;
                        expected[end - start + 1] = 0x01;
                        let actual = &strips[0][start - 1..=end];
                        for (index, slope) in [
                            (0, boundary_slopes[0]),
                            (expected.len() - 1, boundary_slopes[1]),
                        ] {
                            let tile = actual[index];
                            // Where land meets water, a ledge or shallow water also bounds a
                            // specialized variant's opening.
                            if tile == slope
                                || tile == slope + 8
                                || (specialized && ((0x28..=0x2f).contains(&tile) || tile == 0x09))
                            {
                                expected[index] = tile;
                            }
                        }
                        // Where a lake's land meets its shallow water depends on neighboring
                        // layers, so that end of the land opening is unchecked.
                        if variant == "lake" && opening == 0 {
                            let last = expected.len() - 1;
                            expected[last] = actual[last];
                        }
                        for (depth, strip) in strips.iter().enumerate() {
                            let (first, last, expected) = if depth == 0 {
                                (start - 1, end, expected.as_slice())
                            } else {
                                (start, end - 1, &expected[1..expected.len() - 1])
                            };
                            let actual = &strip[first..=last];
                            let length = actual.len();
                            if actual.iter().zip(expected).enumerate().all(
                                |(index, (&actual, &expected))| {
                                    actual == expected
                                        || (depth > 0 && actual == 0x09 && expected != 0x01)
                                        // Banks and ledges may narrow a specialized opening
                                        // diagonally inward, by a tile per strip after the first.
                                        || (specialized
                                            && (index + 1 < depth || index + depth > length)
                                            && ((0x10..=0x1f).contains(&actual)
                                                || (0x28..=0x2f).contains(&actual)))
                                },
                            ) {
                                continue;
                            }
                            let key = (area_id + world, slot.name.clone(), variant.clone(), start);
                            if aperture_failures
                                .get(&key)
                                .is_none_or(|(previous_depth, _, _)| depth < *previous_depth)
                            {
                                let message = format!(
                                    "${:02X} {} {}={variant}, {depth} tiles inward, tiles {first}..={last}: expected {expected:02x?}, got {actual:02x?}",
                                    area_id + world,
                                    area_name,
                                    slot.name,
                                );
                                aperture_failures.insert(key, (depth, message, edges.clone()));
                            }
                            break;
                        }
                    }
                }
                combined.extend_from_slice(&collision);
            }
            configurations += 1;
            let fingerprint = Sha256::digest(&combined).into();
            // A one-slot difference is already globally minimal for this room.
            if duplicate.as_ref().is_none_or(|(count, _)| *count > 1) {
                let previous_combinations = seen.entry(fingerprint).or_default();
                for previous in previous_combinations.iter() {
                    let mut differences = 0;
                    let mut different_geometry = false;
                    for ((_, before), (_, after)) in previous.iter().zip(&edges) {
                        if before != after {
                            differences += 1;
                            if !matches!(
                                (before.as_str(), after.as_str()),
                                ("light-only", "light-and-mirror")
                                    | ("light-and-mirror", "light-only")
                                    | ("dual-small", "dual-upper")
                                    | ("dual-upper", "dual-small")
                            ) {
                                different_geometry = true;
                            }
                        }
                    }
                    if different_geometry
                        && duplicate
                            .as_ref()
                            .is_none_or(|(count, _)| differences < *count)
                    {
                        duplicate = Some((differences, [previous.clone(), edges.clone()]));
                        if differences == 1 {
                            break;
                        }
                    }
                }
                previous_combinations.push(edges.clone());
            }
            for index in (0..selection.len()).rev() {
                selection[index] += 1;
                if selection[index] < slots[index].variants.len() {
                    continue 'combinations;
                }
                selection[index] = 0;
            }
            break;
        }
        if let Some((differences, [first, second])) = duplicate {
            failures.insert(
                format!(
                    "{}: identical collision layers for {first:?} and {second:?} ({differences} differing slots)",
                    room.name
                ),
                second,
            );
        }
    }
    for (_, message, example) in aperture_failures.into_values() {
        failures.insert(message, example);
    }
    eprintln!("Checked {configurations} paired-world configurations and {apertures} apertures.");
    for (message, example) in &failures {
        eprintln!("{message}\n  example: {example:?}\n");
    }
    assert!(
        failures.is_empty(),
        "{} distinct geometry failures",
        failures.len()
    );
    Ok(())
}
