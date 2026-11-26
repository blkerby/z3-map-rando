use anyhow::Result;
use generator::{inputs::prepare_skill_inputs, state::GlobalState, strat_check::check_room_strats};
use logic_catalog::RoomIndex;
use std::{collections::BTreeMap, fs::File, io::BufWriter, path::Path};

/// Write `logic_data.json`, the data the Logic pages show beyond the room files: each strat's
/// difficulty, keyed by room ID and then strat ID. A strat's difficulty is the label of the first
/// skill preset, easiest first, with which it passes (see `check_room_strats`), or "Ignored".
pub fn build_logic_data(logic_catalog: &Path, preset_catalog: &Path, output: &Path) -> Result<()> {
    let catalog = logic_catalog::decode_catalog(File::open(logic_catalog)?)?;
    let presets = preset_catalog::decode_catalog(File::open(preset_catalog)?)?;
    let global = GlobalState::initialize_full(&catalog);
    let mut skills = Vec::new();
    for preset in &presets.skill {
        skills.push((
            preset.label.as_str(),
            prepare_skill_inputs(&catalog, &preset.settings)?,
        ));
    }
    let mut difficulties = BTreeMap::new();
    for (index, room) in catalog.rooms.iter().enumerate() {
        let Some(room_id) = room.room_id else {
            continue;
        };
        let mut room_difficulties = BTreeMap::new();
        for (label, skill) in &skills {
            let passes = check_room_strats(&catalog, &global, skill, RoomIndex(index as u32));
            for (strat, passes) in room.strats.iter().zip(passes) {
                if let (Some(strat_id), true) = (strat.strat_id, passes) {
                    room_difficulties.entry(strat_id).or_insert(*label);
                }
            }
        }
        for strat in &room.strats {
            if let Some(strat_id) = strat.strat_id {
                room_difficulties.entry(strat_id).or_insert("Ignored");
            }
        }
        difficulties.insert(room_id, room_difficulties);
    }
    serde_json::to_writer(BufWriter::new(File::create(output)?), &difficulties)?;
    Ok(())
}
