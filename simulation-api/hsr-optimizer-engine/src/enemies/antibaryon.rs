/**
 * @enemy Antibaryon
 * @id 50cf7b6b-c373-4ee8-ace8-13bf101e0f0f
 * @ability Obliterate: Deals Imaginary DMG (250% ATK) to a single target.
 */

use crate::ids;
use crate::models::SimState;

/// Returns `Some((damage, log_message))` for the simulator to apply via `apply_damage_to_ally`.
/// Returns `None` if the attack cannot proceed (no valid target).
pub fn on_action(
    state: &SimState,
    e_idx: usize,
    target_ally_idx: usize,
) -> Option<(f64, String)> {
    let enemy = state.enemies[e_idx].as_ref()?;
    let target = state.team.get(target_ally_idx)?;
    if target.is_downed { return None; }

    let enemy_atk = enemy.base_stats.get(ids::ENEMY_ATK_ID).copied().unwrap_or(500.0);
    let target_def = target.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(600.0);

    let base_dmg = enemy_atk * 2.5;
    let enemy_lv = enemy.level as f64;
    let def_mult = (enemy_lv * 10.0 + 200.0) / (target_def + enemy_lv * 10.0 + 200.0);
    let damage   = (base_dmg * def_mult).floor();

    let log = format!(
        "Obliterate (Imaginary) on {} -> {:.0} DMG",
        target.name, damage
    );
    Some((damage, log))
}

pub fn on_battle_start(_state: &mut SimState, _e_idx: usize) {}
pub fn on_turn_start(_state: &mut SimState, _e_idx: usize) {}
