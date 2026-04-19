mod antibaryon;
mod baryon;

use crate::ids;
use crate::models::SimState;

// ─── on_battle_start ─────────────────────────────────────────────────────────

pub fn dispatch_on_battle_start(state: &mut SimState, e_idx: usize) {
    let kit_id = match state.enemies[e_idx].as_ref() {
        Some(e) => e.kit_id.clone(),
        None    => return,
    };
    match kit_id.as_str() {
        ids::ANTIBARYON_ID => antibaryon::on_battle_start(state, e_idx),
        ids::BARYON_ID     => baryon::on_battle_start(state, e_idx),
        _                  => {}
    }
}

// ─── on_turn_start ───────────────────────────────────────────────────────────

pub fn dispatch_on_turn_start(state: &mut SimState, e_idx: usize) {
    let kit_id = match state.enemies[e_idx].as_ref() {
        Some(e) => e.kit_id.clone(),
        None    => return,
    };
    match kit_id.as_str() {
        ids::ANTIBARYON_ID => antibaryon::on_turn_start(state, e_idx),
        ids::BARYON_ID     => baryon::on_turn_start(state, e_idx),
        _                  => {}
    }
}

// ─── on_action ───────────────────────────────────────────────────────────────

/// Returns `Some((damage, log_message))` when a kit handles the action.
/// Returns `None` for unknown enemies — simulator should use the generic fallback.
pub fn dispatch_on_action(
    state: &SimState,
    e_idx: usize,
    target_ally_idx: usize,
) -> Option<(f64, String)> {
    let kit_id = state.enemies[e_idx].as_ref()?.kit_id.clone();
    match kit_id.as_str() {
        ids::ANTIBARYON_ID => antibaryon::on_action(state, e_idx, target_ally_idx),
        ids::BARYON_ID     => baryon::on_action(state, e_idx, target_ally_idx),
        _                  => None, // generic fallback handled by simulator
    }
}
