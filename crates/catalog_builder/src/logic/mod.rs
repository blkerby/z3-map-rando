mod connections;
mod requirements;
mod rooms;

use crate::z3_json_data::{self, collect_source_files, read_source};
use anyhow::Result;
use logic_catalog::*;
use requirements::Compiler;
use std::{
    collections::BTreeMap,
    fs,
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Default)]
struct RoomIndices {
    dungeon_idx: Option<DungeonIndex>,
    vertices: BTreeMap<(u32, Option<World>), VertexIndex>,
    entrances: BTreeMap<(u32, World), EntranceIndex>,
    teleports: BTreeMap<(u32, World), TeleportIndex>,
    whirlpools: BTreeMap<(u32, World), WhirlpoolIndex>,
    items: BTreeMap<u32, Effect>,
    doors: BTreeMap<u32, DoorIndex>,
    obstacles: BTreeMap<String, ObstacleIndex>,
}

fn collect_techs(
    techs: Vec<z3_json_data::Tech>,
    catalog: &mut LogicCatalog,
    compiler: &mut Compiler,
) {
    for mut tech in techs {
        let extensions = std::mem::take(&mut tech.extension_techs);
        let tech_idx = TechIndex(catalog.techs.len() as u32);
        catalog.techs.push(TechDefinition {
            tech_id: tech.id,
            name: tech.name.clone(),
            tiers: std::mem::take(&mut tech.tiers),
        });
        compiler.techs.insert(tech.name.clone(), (tech_idx, tech));
        collect_techs(extensions, catalog, compiler);
    }
}

/// Compile z3-json-data and write the encoded logic catalog.
pub fn build_catalog(
    source_directory: &Path,
    output_catalog: &Path,
    compression_level: i32,
) -> Result<()> {
    let mut catalog = LogicCatalog {
        vertices: Vec::new(),
        vertex_metadata: Vec::new(),
        edges: Vec::new(),
        edge_metadata: Vec::new(),
        item_locations: Vec::new(),
        start_vertex_idx: VertexIndex(0),
        rooms: Vec::new(),
        dungeons: Vec::new(),
        items: Vec::new(),
        techs: Vec::new(),
        flags: Vec::new(),
        doors: Vec::new(),
        events: Vec::new(),
        entrances: Vec::new(),
        screen_boundaries: Vec::new(),
        teleports: Vec::new(),
        whirlpools: Vec::new(),
        flute_spots: Vec::new(),
        flute_hub_vertex_idx: VertexIndex(0),
        vanilla_connections: Vec::new(),
        event_entries: Vec::new(),
        event_exits: Vec::new(),
    };
    let compiler = load_definitions(source_directory, &mut catalog)?;
    let room_indices = rooms::compile_rooms(source_directory, &mut catalog, &compiler)?;
    connections::build_connections(source_directory, &room_indices, &mut catalog)?;
    build_flute_travel(&mut catalog, compiler.items["OcarinaActive"]);
    write_catalog(&catalog, output_catalog, compression_level)
}

fn build_flute_travel(catalog: &mut LogicCatalog, active_flute_idx: ItemIndex) {
    let room_idx = RoomIndex(catalog.rooms.len() as u32);
    catalog.rooms.push(Room {
        room_id: None,
        name: "Flute travel".to_owned(),
        kind: RoomKind::FluteHub,
        dungeon_idx: None,
        position: None,
        size: None,
        obstacles: Vec::new(),
        nodes: vec![NodeMetadata {
            node_id: 0,
            name: "Flute hub".to_owned(),
            node_type: None,
        }],
        strats: Vec::new(),
    });
    catalog.flute_hub_vertex_idx = VertexIndex(catalog.vertices.len() as u32);
    catalog.vertices.push(Vertex { room_idx });
    catalog.vertex_metadata.push(VertexMetadata {
        node_idx: NodeIndex(0),
        world: None,
        origin: VertexOrigin::Node,
    });

    for (index, vertex) in catalog.vertices.iter().enumerate() {
        let vertex_idx = VertexIndex(index as u32);
        let metadata = catalog.vertex_metadata(vertex_idx);
        if catalog.room(vertex.room_idx).kind != RoomKind::Overworld
            || metadata.world != Some(World::Light)
            || !matches!(metadata.origin, VertexOrigin::Node)
        {
            continue;
        }
        catalog.edges.push(Edge {
            from_vertex_idx: vertex_idx,
            to_vertex_idx: catalog.flute_hub_vertex_idx,
            requirement: Requirement::Item(active_flute_idx),
            effects: Box::new([]),
        });
        catalog.edge_metadata.push(EdgeMetadata {
            origin: EdgeOrigin::ImplicitFlute,
        });
    }
    for spot in &catalog.flute_spots {
        catalog.edges.push(Edge {
            from_vertex_idx: catalog.flute_hub_vertex_idx,
            to_vertex_idx: spot.vertex_idx,
            requirement: Requirement::Free,
            effects: Box::new([]),
        });
        catalog.edge_metadata.push(EdgeMetadata {
            origin: EdgeOrigin::ImplicitFlute,
        });
    }
}

fn load_definitions(source_directory: &Path, catalog: &mut LogicCatalog) -> Result<Compiler> {
    let mut compiler = Compiler {
        items: BTreeMap::new(),
        flags: BTreeMap::new(),
        helpers: BTreeMap::new(),
        techs: BTreeMap::new(),
        damage: BTreeMap::new(),
    };
    let items: z3_json_data::Items = read_source(&source_directory.join("items.json"))?;
    for category in [
        items.inventory,
        items.refills,
        items.bottle_contents,
        items.currency,
        items.dungeon_items,
        items.dungeon_prizes,
        items.goal_items,
        items.expansions,
    ] {
        for item in category {
            let item_idx = ItemIndex(catalog.items.len() as u32);
            compiler.items.insert(item.name.clone(), item_idx);
            catalog.items.push(ItemDefinition { name: item.name });
        }
    }
    for name in items.flags {
        let flag_idx = FlagIndex(catalog.flags.len() as u32);
        compiler.flags.insert(name.clone(), flag_idx);
        catalog.flags.push(name);
    }
    let helpers: Vec<z3_json_data::Helper> = read_source(&source_directory.join("helpers.json"))?;
    for helper in helpers {
        compiler.helpers.insert(helper.name.clone(), helper);
    }
    let techs: z3_json_data::Techs = read_source(&source_directory.join("tech.json"))?;
    for category in techs.tech_categories {
        collect_techs(category.techs, catalog, &mut compiler);
    }
    let mut enemy_paths = Vec::new();
    collect_source_files(&source_directory.join("enemies"), &mut enemy_paths)?;
    enemy_paths.sort();
    for path in enemy_paths {
        let enemies: z3_json_data::Enemies = read_source(&path)?;
        for enemy in enemies.enemies {
            let damage = [
                enemy.dmg_to_link.green,
                enemy.dmg_to_link.blue,
                enemy.dmg_to_link.red,
            ];
            for name in enemy.names {
                compiler.damage.insert(name, damage);
            }
        }
    }

    Ok(compiler)
}

fn write_catalog(
    catalog: &LogicCatalog,
    output_catalog: &Path,
    compression_level: i32,
) -> Result<()> {
    let mut output = BufWriter::new(fs::File::create(output_catalog)?);
    encode_catalog(catalog, &mut output, compression_level)?;
    output.flush()?;
    let bytes_len = output.get_ref().metadata()?.len();
    eprintln!(
        "Wrote logic catalog to {} ({} bytes)\n  {} rooms, {} vertices, {} edges, {} item locations, {} vanilla connections",
        output_catalog.display(),
        bytes_len,
        catalog.rooms.len(),
        catalog.vertices.len(),
        catalog.edges.len(),
        catalog.item_locations.len(),
        catalog.vanilla_connections.len(),
    );
    Ok(())
}
