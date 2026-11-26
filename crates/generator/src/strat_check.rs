//! Check strats against skill settings alone, as shown on the Logic pages.

use crate::{
    inputs::SkillInputs,
    interpreter::{GlobalDiscoveries, Interpreter},
    local_state_reducer::LocalStateReducer,
    state::{GlobalState, LocalState},
};
use logic_catalog::{EdgeOrigin, FollowerMask, LogicCatalog, RoomIndex};

/// Whether each strat of a room passes with the skill settings, indexed by StratIndex.
///
/// `global` normally owns everything (`GlobalState::initialize_full`), so that only the skill
/// settings decide. Each strat starts from unused resources with any follower possible, and
/// passes if one of its edges succeeds for some combination of cleared room obstacles.
pub fn check_room_strats(
    catalog: &LogicCatalog,
    global: &GlobalState,
    skill: &SkillInputs,
    room_idx: RoomIndex,
) -> Vec<bool> {
    let interpreter = Interpreter {
        catalog,
        skill,
        global,
    };
    let room = catalog.room(room_idx);
    let mut passes = vec![false; room.strats.len()];
    // Discoveries don't matter with everything owned.
    let mut discoveries = GlobalDiscoveries::default();
    for (edge, metadata) in catalog.edges.iter().zip(&catalog.edge_metadata) {
        let EdgeOrigin::Strat {
            room_idx: edge_room_idx,
            strat_idx,
        } = metadata.origin
        else {
            continue;
        };
        if edge_room_idx != room_idx || passes[strat_idx.0 as usize] {
            continue;
        }
        for cleared_obstacles in 0..1_u16 << room.obstacles.len() {
            let mut state = LocalState::initialize();
            // Bits beyond the defined followers are never read.
            state.possible_followers = FollowerMask(u8::MAX);
            state.cleared_obstacles = cleared_obstacles as u8;
            let mut states = LocalStateReducer::default();
            states.insert(state);
            let outputs =
                interpreter.evaluate_requirement(&edge.requirement, states, &mut discoveries);
            if !outputs.get_states().is_empty() {
                passes[strat_idx.0 as usize] = true;
                break;
            }
        }
    }
    passes
}
