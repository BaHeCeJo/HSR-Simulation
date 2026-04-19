//! Lightcone passive dispatch.
//!
//! Each function mirrors the character hook with the same name.
//! The simulator calls these immediately after the corresponding character hook.
//!
//! Adding a new LC:
//!   1. Create `src/lightcones/<name>.rs` with the four pub hook functions.
//!   2. Add `mod <name>;` below.
//!   3. Add the LC ID constant to `ids.rs`.
//!   4. Add a match arm in each dispatcher.

mod dream_scented_wheat;
mod passing_shore;

use crate::ids;
use crate::models::{ActionParams, SimState};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Dispatch guard: returns the LC id only if it is non-empty.
#[inline]
fn lc_id(state: &SimState, idx: usize) -> &str {
    &state.team[idx].lightcone.id
}

// ─── Dispatchers ─────────────────────────────────────────────────────────────

pub fn dispatch_on_battle_start(state: &mut SimState, idx: usize) {
    match lc_id(state, idx) {
        ids::LC_DREAM_SCENTED_WHEAT_ID     => dream_scented_wheat::on_battle_start(state, idx),
        ids::LC_ALONG_THE_PASSING_SHORE_ID => passing_shore::on_battle_start(state, idx),
        _ => {}
    }
}

pub fn dispatch_on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    match lc_id(state, idx) {
        ids::LC_DREAM_SCENTED_WHEAT_ID     => dream_scented_wheat::on_before_action(state, idx, action, target_idx),
        ids::LC_ALONG_THE_PASSING_SHORE_ID => passing_shore::on_before_action(state, idx, action, target_idx),
        _ => {}
    }
}

pub fn dispatch_on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    match lc_id(state, idx) {
        ids::LC_DREAM_SCENTED_WHEAT_ID     => dream_scented_wheat::on_after_action(state, idx, action, target_idx),
        ids::LC_ALONG_THE_PASSING_SHORE_ID => passing_shore::on_after_action(state, idx, action, target_idx),
        _ => {}
    }
}
