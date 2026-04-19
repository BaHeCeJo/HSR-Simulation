use crate::effects;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy        = 110.0;
    state.team[idx].buffs.atk_percent += 18.0;  // minor trace: ATK +18%
    state.team[idx].buffs.dmg_boost   += 10.0;  // minor trace: Ice DMG +10%
    state.team[idx].buffs.effect_res  += 18.0;  // minor trace: Effect RES +18%
}

pub fn on_turn_start(_state: &mut SimState, _idx: usize) {}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Ultimate => {
            action.inflicts_debuff = true;
            let ehr = state.team[idx].base_stats.get(crate::ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
                    + state.team[idx].buffs.effect_hit_rate;
            // Pela's Ult is AoE — apply DEF reduction to ALL enemies.
            for slot in state.enemies.iter_mut() {
                if let Some(enemy) = slot.as_mut() {
                    effects::try_apply_enemy_debuff(ehr, enemy, "pela_ult_def", StatusEffect {
                        duration: 2,
                        value:    40.0,
                        stat:     Some("DEF Reduction".to_string()),
                        effects:  vec![],
                    }, 1.0);
                }
            }
        }
        ActionType::Skill => {
            action.inflicts_debuff = true;
        }
        _ => {}
    }
    let _ = (idx, target_idx);
}

pub fn on_after_action(
    _state: &mut SimState,
    _idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}

pub fn on_ult(_state: &mut SimState, _idx: usize) {}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
