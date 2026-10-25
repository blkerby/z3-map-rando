//! Fixed cost metrics for selecting representative route-local states.

use crate::state::{HealthState, LocalState};
use logic_catalog::Follower;

// Each row defines a different way to score a LocalState. Keeping the best state
// for each row limits the retained set to 18 states while preserving alternatives
// that conserve different resources, keep different obstacles cleared, or retain
// different followers.
//
// Columns: health, magic, bombs, arrows, bottles; obstacle bits 0..3;
// Zelda, Old Man, Blind, Dwarf, Purple Chest, Super Bomb.
// Resource values 0..3 give decreasing priority to conserving that resource.
// Obstacle values prefer uncleared (0) or cleared (1).
// Follower values give a stronger (0) or weaker (1) preference for having that follower.
//
// The rows include every combination of settings for any two columns. For any
// three columns, they include every low/high combination, grouping resource
// values 0/1 as low and 2/3 as high. This spreads the metrics across tradeoffs
// between state components without needing a metric for every full combination.
const COVERING_ARRAY: [[u8; 15]; 18] = [
    [1, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [2, 0, 3, 2, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 1],
    [3, 1, 2, 2, 2, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0],
    [1, 2, 2, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0],
    [3, 2, 3, 3, 0, 1, 0, 0, 1, 1, 0, 0, 0, 1, 1],
    [0, 3, 3, 1, 2, 0, 0, 0, 1, 0, 0, 1, 1, 0, 1],
    [1, 0, 0, 3, 2, 1, 0, 0, 1, 1, 1, 1, 0, 1, 0],
    [2, 3, 2, 3, 3, 1, 0, 1, 0, 0, 0, 1, 1, 0, 0],
    [0, 0, 2, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1],
    [3, 0, 1, 1, 3, 1, 0, 1, 0, 1, 1, 0, 0, 0, 1],
    [2, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 0],
    [3, 3, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0],
    [1, 1, 3, 2, 3, 0, 0, 1, 1, 0, 1, 0, 1, 1, 1],
    [0, 2, 0, 0, 3, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1],
    [0, 3, 0, 2, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 1],
    [0, 1, 1, 3, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0],
    [2, 1, 3, 0, 2, 0, 1, 0, 1, 1, 1, 0, 1, 0, 0],
    [2, 2, 1, 2, 2, 0, 1, 1, 1, 0, 1, 1, 0, 1, 1],
];

pub const METRIC_COUNT: usize = COVERING_ARRAY.len();

const NUM_RESOURCE_LEVELS: usize = 4;
const RESOURCE_SCALES: [usize; 5] = [160, 128, 50, 70, 4];
const FOLLOWERS: [Follower; 6] = [
    Follower::Zelda,
    Follower::OldMan,
    Follower::Blind,
    Follower::Dwarf,
    Follower::PurpleChest,
    Follower::SuperBomb,
];
const SCORE_SCALE: i32 = 1 << 24;
// Ratio between the strongest and weakest weight for each component.
// Also sets the exponential growth in cost over a resource's full scale.
const WEIGHT_RANGE_RATIO: f64 = 8.0;
const FOLLOWER_COST: i32 = ((WEIGHT_RANGE_RATIO - 1.0) * SCORE_SCALE as f64 + 0.5) as i32;
const OBSTACLE_COST: i32 =
    ((WEIGHT_RANGE_RATIO + 1.0 / WEIGHT_RANGE_RATIO - 2.0) * SCORE_SCALE as f64 + 0.5) as i32;

const LN_WEIGHT_RANGE_RATIO: f64 = compute_log(WEIGHT_RANGE_RATIO);

// Rust doesn't have a const implementation of `log` or `exp` so we implement them
// from scratch for compile-time use.
const fn compute_log(value: f64) -> f64 {
    // For value >= 1, reduce to m in [1, 2), then use ln(value) = k * ln(2) + ln(m).
    // With x = (m - 1) / (m + 1) < 1/3, 16 terms of ln(m) = 2 * (x + x^3/3 + ...)
    // suffice for approximately f64 precision, even when value is large.
    let mut reduced = value;
    let mut exponent = 0;
    while reduced >= 2.0 {
        reduced *= 0.5;
        exponent += 1;
    }
    let x = (reduced - 1.0) / (reduced + 1.0);
    let mut power = x;
    let mut sum = 0.0;
    let mut n = 1;
    while n < 32 {
        sum += power / n as f64;
        power *= x * x;
        n += 2;
    }
    exponent as f64 * std::f64::consts::LN_2 + 2.0 * sum
}

const fn compute_exp(exponent: f64) -> f64 {
    // Write exp(x) = 2^k * exp(r), keeping |r| <= ln(2)/2 for fast convergence.
    let mut power = (exponent / std::f64::consts::LN_2).round() as i32;
    let reduced = exponent - power as f64 * std::f64::consts::LN_2;
    let mut sum = 1.0;
    let mut term = 1.0;
    let mut n = 1;
    while n <= 16 {
        term *= reduced / n as f64;
        sum += term;
        n += 1;
    }
    while power > 0 {
        sum *= 2.0;
        power -= 1;
    }
    while power < 0 {
        sum *= 0.5;
        power += 1;
    }
    sum
}

// For B = WEIGHT_RANGE_RATIO, L = NUM_RESOURCE_LEVELS, target level t, and finite usage u,
// store round(SCORE_SCALE * B^(-t/(L - 1)) * (B^(u/scale) - 1)).
// Subtracting the constant term leaves every metric's ordering unchanged.
static RESOURCE_COSTS: [[[i32; 256]; NUM_RESOURCE_LEVELS]; 5] = {
    let mut costs = [[[0; 256]; NUM_RESOURCE_LEVELS]; 5];
    let weight_step = compute_exp(-LN_WEIGHT_RANGE_RATIO / (NUM_RESOURCE_LEVELS - 1) as f64);
    let mut resource = 0;
    while resource < 5 {
        let factor = compute_exp(LN_WEIGHT_RANGE_RATIO / RESOURCE_SCALES[resource] as f64);
        let mut exponential = 1.0;
        let mut usage = 0;
        while usage <= RESOURCE_SCALES[resource] {
            let mut target = 0;
            let mut weight = 1.0;
            while target < NUM_RESOURCE_LEVELS {
                costs[resource][target][usage] =
                    ((exponential - 1.0) * SCORE_SCALE as f64 * weight + 0.5) as i32;
                weight *= weight_step;
                target += 1;
            }
            exponential *= factor;
            usage += 1;
        }
        resource += 1;
    }
    costs
};

// An obstacle/complement pair is equivalent, up to a constant, to +/- (B + 1/B - 2)
// times the obstacle bit. A present follower receives a bonus of (B - 1) * B^-target.
static BINARY_COSTS: [[i32; 10]; METRIC_COUNT] = {
    let mut costs = [[0; 10]; METRIC_COUNT];
    let mut metric = 0;
    while metric < METRIC_COUNT {
        let mut obstacle = 0;
        while obstacle < 4 {
            costs[metric][obstacle] = if COVERING_ARRAY[metric][5 + obstacle] == 0 {
                OBSTACLE_COST
            } else {
                -OBSTACLE_COST
            };
            obstacle += 1;
        }
        let mut follower = 0;
        while follower < FOLLOWERS.len() {
            costs[metric][4 + follower] = if COVERING_ARRAY[metric][9 + follower] == 0 {
                -FOLLOWER_COST
            } else {
                -((WEIGHT_RANGE_RATIO - 1.0) / WEIGHT_RANGE_RATIO * SCORE_SCALE as f64 + 0.5) as i32
            };
            follower += 1;
        }
        metric += 1;
    }
    costs
};

/// Compute all metric scores; lower is better, with each score scaled by 2^24.
/// Compare states at the same vertex or requirement position within one traversal wave.
/// With the current weight range, scores omit constants and stay within [-66.5, 59.5] * 2^24.
pub fn compute_costs(state: &LocalState) -> [i32; METRIC_COUNT] {
    let usages = [
        match state.health {
            HealthState::Consumed(used) => used,
            // Score fixed health pessimistically against the maximum capacity.
            HealthState::Remaining(remaining) => 160 - remaining,
        },
        state.magic_used,
        state.bombs_used,
        state.arrows_used,
        state.bottles_used,
    ];
    let mut costs = [0; METRIC_COUNT];
    for metric in 0..METRIC_COUNT {
        for resource in 0..5 {
            costs[metric] += RESOURCE_COSTS[resource][COVERING_ARRAY[metric][resource] as usize]
                [usages[resource] as usize];
        }
        for obstacle in 0..4 {
            if state.cleared_obstacles & (1 << obstacle) != 0 {
                costs[metric] += BINARY_COSTS[metric][obstacle];
            }
        }
        for follower in 0..FOLLOWERS.len() {
            if state.possible_followers.0 & (1 << FOLLOWERS[follower] as u8) != 0 {
                costs[metric] += BINARY_COSTS[metric][4 + follower];
            }
        }
    }
    costs
}
