//! Keep representative local states by retaining the winner of each cost metric.

use crate::{
    cost_metrics::{METRIC_COUNT, compute_costs},
    state::LocalState,
};

/// Retain at most METRIC_COUNT distinct states, keeping the incumbent on ties.
/// Compare states only at the same vertex or requirement position within one traversal wave.
#[derive(Clone, Debug, Default)]
pub struct LocalStateReducer {
    states: Vec<LocalState>,
    best_costs: [i32; METRIC_COUNT],
    best_state_indices: [u8; METRIC_COUNT],
}

impl LocalStateReducer {
    /// Insert a state if it improves any metric, removing states that no longer win.
    /// Return whether the state was retained. Stored indices may change after insertion.
    pub fn insert(&mut self, state: LocalState) -> bool {
        let costs = compute_costs(&state);
        let is_empty = self.states.is_empty();
        let state_idx = self.states.len() as u8;
        let mut improved = false;
        for metric in 0..METRIC_COUNT {
            if is_empty || costs[metric] < self.best_costs[metric] {
                self.best_costs[metric] = costs[metric];
                self.best_state_indices[metric] = state_idx;
                improved = true;
            }
        }
        if !improved {
            return false;
        }
        self.states.push(state);

        // One extra slot holds the new state until obsolete winners are removed.
        let mut retained = [false; METRIC_COUNT + 1];
        for &idx in &self.best_state_indices {
            retained[idx as usize] = true;
        }
        let mut new_indices = [0_u8; METRIC_COUNT + 1];
        let mut old_idx = 0;
        let mut new_idx = 0;
        self.states.retain(|_| {
            let keep = retained[old_idx];
            if keep {
                new_indices[old_idx] = new_idx;
                new_idx += 1;
            }
            old_idx += 1;
            keep
        });
        for idx in &mut self.best_state_indices {
            *idx = new_indices[*idx as usize];
        }
        true
    }

    /// Return the retained states without exposing mutable access to the metric winners.
    pub fn get_states(&self) -> &[LocalState] {
        &self.states
    }

    /// Consume the reducer to process its states without keeping stale metric scores.
    pub fn into_states(self) -> Vec<LocalState> {
        self.states
    }
}
