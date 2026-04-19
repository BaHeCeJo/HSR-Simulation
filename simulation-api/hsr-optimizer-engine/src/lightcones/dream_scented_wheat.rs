//! A Dream Scented in Wheat  (Erudition, ID: df8260b3-…)
//!
//! Passive — all values by superimposition 1/2/3/4/5:
//!   CRIT Rate  +12 / 14 / 16 / 18 / 20 %
//!   Ult & FUP DMG  +24 / 28 / 32 / 36 / 40 %

use crate::models::{ActionParams, ActionType, SimState};

const CR_TABLE:  [f64; 5] = [12.0, 14.0, 16.0, 18.0, 20.0];
const DMG_TABLE: [f64; 5] = [24.0, 28.0, 32.0, 36.0, 40.0];

#[inline]
fn si_idx(si: i32) -> usize { ((si - 1).clamp(0, 4)) as usize }

/// Apply the permanent CRIT Rate bonus at battle start.
pub fn on_battle_start(state: &mut SimState, idx: usize) {
    let si = state.team[idx].lightcone.superimposition;
    state.team[idx].buffs.crit_rate += CR_TABLE[si_idx(si)];
}

/// Add the DMG boost for Ultimate and Follow-up ATK actions.
/// This fires inside the buffs-snapshot window so the boost is automatically
/// reverted after the action completes — no manual cleanup needed.
pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    if matches!(action.action_type, ActionType::Ultimate | ActionType::FollowUp) {
        let si = state.team[idx].lightcone.superimposition;
        state.team[idx].buffs.dmg_boost += DMG_TABLE[si_idx(si)];
    }
}

pub fn on_after_action(
    _state: &mut SimState,
    _idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
