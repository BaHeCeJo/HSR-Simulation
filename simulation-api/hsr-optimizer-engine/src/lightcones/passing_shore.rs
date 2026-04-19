//! Along the Passing Shore  (Nihility, ID: 8a6a5884-…)
//!
//! Passive — all values by superimposition 1/2/3/4/5:
//!   CRIT DMG           +36 / 42 / 48 / 54 / 60 %
//!   DMG to Mirage Fizzle targets  +24 / 28 / 32 / 36 / 40 %
//!   Ult DMG additionally          +24 / 28 / 32 / 36 / 40 %
//!
//! Mirage Fizzle is a 1-turn debuff applied to the target after every hit.
//! The wearer deals the bonus DMG to any target that already has Mirage Fizzle.
//! Because the debuff is applied *after* the hit, the first hit on a fresh target
//! has no bonus; every subsequent hit refreshes the debuff and gains the bonus.

use crate::effects;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

const CD_TABLE:  [f64; 5] = [36.0, 42.0, 48.0, 54.0, 60.0];
const DMG_TABLE: [f64; 5] = [24.0, 28.0, 32.0, 36.0, 40.0];

const MIRAGE_FIZZLE: &str = "mirage_fizzle";

#[inline]
fn si_idx(si: i32) -> usize { ((si - 1).clamp(0, 4)) as usize }

/// Apply the permanent CRIT DMG bonus at battle start.
pub fn on_battle_start(state: &mut SimState, idx: usize) {
    let si = state.team[idx].lightcone.superimposition;
    state.team[idx].buffs.crit_dmg += CD_TABLE[si_idx(si)];
}

/// If the primary target already has Mirage Fizzle:
///   • Add the base DMG bonus (all action types).
///   • Add the same bonus again if this is an Ultimate (additional stacking).
///
/// These boosts sit inside the buffs-snapshot window so they auto-revert
/// after the action — no manual cleanup needed.
pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    if !matches!(
        action.action_type,
        ActionType::Basic | ActionType::Skill | ActionType::Ultimate | ActionType::FollowUp
    ) {
        return;
    }

    let has_fizzle = target_idx
        .and_then(|t| state.enemies.get(t))
        .and_then(|slot| slot.as_ref())
        .map(|e| e.active_debuffs.contains_key(MIRAGE_FIZZLE))
        .unwrap_or(false);

    if has_fizzle {
        let si      = state.team[idx].lightcone.superimposition;
        let dmg_val = DMG_TABLE[si_idx(si)];
        state.team[idx].buffs.dmg_boost += dmg_val;
        // Ult additionally benefits (total ult bonus = dmg_val × 2)
        if action.action_type == ActionType::Ultimate {
            state.team[idx].buffs.dmg_boost += dmg_val;
        }
    }
}

/// Inflict Mirage Fizzle on the target after every hitting action.
/// Duration 1 means it expires on the enemy's next turn tick.
/// Re-applying refreshes the duration, keeping it active as long as the
/// wearer keeps attacking.
pub fn on_after_action(
    state: &mut SimState,
    _idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    if !matches!(
        action.action_type,
        ActionType::Basic | ActionType::Skill | ActionType::Ultimate | ActionType::FollowUp
    ) {
        return;
    }
    if let Some(t) = target_idx {
        if let Some(enemy) = state.enemies.get_mut(t).and_then(|s| s.as_mut()) {
            effects::apply_enemy_debuff(enemy, MIRAGE_FIZZLE, StatusEffect {
                duration: 1,
                value:    0.0,
                stat:     None,
                effects:  vec![],
            });
        }
    }
}
