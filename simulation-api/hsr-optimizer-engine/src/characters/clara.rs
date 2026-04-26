use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ─── Stack keys (static str → TeamMember.stacks) ─────────────────────────────
const ENHANCED_REM: &str = "clara_enhanced_rem";  // remaining enhanced counter uses
const ULT_RED_REM: &str  = "clara_ult_red_rem";   // remaining turns of ult 25% DMG reduction
const E2_ATK_REM: &str   = "clara_e2_atk_rem";    // remaining turns of E2 ATK buff
const E4_ACTIVE: &str    = "clara_e4_active";      // E4 extra 30% reduction active flag

// Per-enemy Mark of Counter (state.stacks → String keys)
fn mark_key(slot: usize) -> String { format!("clara_mark_{slot}") }

// ─── Svarog counter helper ────────────────────────────────────────────────────

fn fire_counter(state: &mut SimState, clara_idx: usize, enemy_slot: usize, enhanced: bool) {
    if state.team[clara_idx].is_downed { return; }
    if state.enemies.get(enemy_slot).and_then(|e| e.as_ref())
        .map_or(true, |e| e.hp <= 0.0) { return; }

    // Multiplier: normal 160%, enhanced = 160% + 160% = 320%
    let mult     = if enhanced { 3.20 } else { 1.60 };
    let toughness = if enhanced { 15.0 } else { 10.0 };

    let mut member = state.team[clara_idx].clone();
    member.buffs.dmg_boost += 30.0; // A6: Counter DMG +30%

    let counter_action = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       mult,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: toughness,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let main_dmg = state.enemies[enemy_slot].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &counter_action))
        .unwrap_or(0.0);

    let mut total_dmg = 0.0;
    if main_dmg > 0.0 {
        if let Some(e) = state.enemies[enemy_slot].as_mut() { e.hp -= main_dmg; }
        total_dmg += main_dmg;
    }
    if state.enemies[enemy_slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[enemy_slot] = None;
    }

    // Enhanced: deal 50% of main DMG to adjacent enemies (blast)
    if enhanced && main_dmg > 0.0 {
        let adj: Vec<usize> = {
            let mut v = Vec::new();
            if enemy_slot > 0 && state.enemies[enemy_slot - 1].is_some() {
                v.push(enemy_slot - 1);
            }
            if enemy_slot + 1 < state.enemies.len() && state.enemies[enemy_slot + 1].is_some() {
                v.push(enemy_slot + 1);
            }
            v
        };
        let adj_dmg = main_dmg * 0.5;
        for &s in &adj {
            if let Some(e) = state.enemies[s].as_mut() { e.hp -= adj_dmg; }
            total_dmg += adj_dmg;
            if state.enemies[s].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[s] = None;
            }
        }
    }

    state.total_damage += total_dmg;

    // Talent energy gain (5 per counter)
    let err = 1.0 + state.team[clara_idx].buffs.energy_regen_rate / 100.0;
    let max_e = state.team[clara_idx].max_energy;
    state.team[clara_idx].energy = (state.team[clara_idx].energy + 5.0 * err).min(max_e);

    let name = state.team[clara_idx].name.clone();
    if enhanced {
        state.add_log(&name, format!("Enhanced Counter (320%+blast): {:.0} DMG", total_dmg));
    } else {
        state.add_log(&name, format!("Svarog Counter (160%): {:.0} DMG", total_dmg));
    }
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 110.0;
    state.team[idx].buffs.atk_percent += 28.0;             // minor trace
    state.team[idx].buffs.dmg_boost   += 14.4;             // Physical DMG +14.4%
    state.team[idx].buffs.hp_percent  += 10.0;             // minor trace
    state.team[idx].buffs.incoming_dmg_reduction += 10.0;  // Talent: always-on reduction
    state.team[idx].aggro_modifier = 1.0;                  // High aggro (Destruction 5 × 2 = 10)

    if state.team[idx].eidolon >= 6 {
        state.stacks.insert("clara_e6_flip".to_string(), 0.0);
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // E4: remove extra 30% reduction (lasted until start of Clara's next turn)
    if state.team[idx].stacks.remove(E4_ACTIVE).is_some() {
        state.team[idx].buffs.incoming_dmg_reduction -= 30.0;
    }

    // Tick ult DMG reduction (2 turns)
    let ult_rem = state.team[idx].stacks.get(ULT_RED_REM).copied().unwrap_or(0.0);
    if ult_rem > 0.0 {
        if ult_rem <= 1.0 {
            state.team[idx].stacks.remove(ULT_RED_REM);
            state.team[idx].buffs.incoming_dmg_reduction -= 25.0;
        } else {
            state.team[idx].stacks.insert(ULT_RED_REM, ult_rem - 1.0);
        }
    }

    // Tick E2 ATK buff (2 turns)
    let e2_rem = state.team[idx].stacks.get(E2_ATK_REM).copied().unwrap_or(0.0);
    if e2_rem > 0.0 {
        if e2_rem <= 1.0 {
            state.team[idx].stacks.remove(E2_ATK_REM);
            state.team[idx].buffs.atk_percent -= 30.0;
        } else {
            state.team[idx].stacks.insert(E2_ATK_REM, e2_rem - 1.0);
        }
    }
}

pub fn on_before_action(
    _state: &mut SimState,
    _idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            action.multiplier       = 1.00;
            action.toughness_damage = 30.0;
        }
        ActionType::Skill => {
            // AoE skill — suppress default single-target hit; handled fully in on_after_action
            action.multiplier       = 0.0;
            action.toughness_damage = 0.0;
        }
        _ => {}
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    _target_idx: Option<usize>,
) {
    if action.action_type != ActionType::Skill { return; }

    let eidolon = state.team[idx].eidolon;

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if alive.is_empty() { return; }

    let member = state.team[idx].clone();

    let skill_action = ActionParams {
        action_type:      ActionType::Skill,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       1.20,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 10.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let mut total_dmg = 0.0;

    // AoE: 120% ATK to all enemies
    for &slot in &alive {
        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &skill_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            total_dmg += dmg;
        }
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[slot] = None;
        }
    }

    // Bonus 120% ATK hit on each marked enemy
    let marked: Vec<usize> = (0..state.enemies.len())
        .filter(|&s| {
            state.stacks.get(&mark_key(s)).copied().unwrap_or(0.0) >= 1.0
                && state.enemies[s].as_ref().map_or(false, |e| e.hp > 0.0)
        })
        .collect();
    let mark_count = marked.len();

    for &slot in &marked {
        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &skill_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            total_dmg += dmg;
        }
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[slot] = None;
        }
    }

    state.total_damage += total_dmg;

    // Remove all marks (E1: marks persist after Skill)
    if eidolon < 1 {
        for s in 0..5 {
            state.stacks.remove(&mark_key(s));
        }
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Skill AoE ({} marked hit): {:.0} DMG", mark_count, total_dmg
    ));
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;

    // Refresh ult DMG reduction (safe on re-cast)
    let old_ult = state.team[idx].stacks.get(ULT_RED_REM).copied().unwrap_or(0.0);
    if old_ult > 0.0 {
        state.team[idx].buffs.incoming_dmg_reduction -= 25.0;
    }
    state.team[idx].buffs.incoming_dmg_reduction += 25.0;
    state.team[idx].stacks.insert(ULT_RED_REM, 2.0);

    // Enhanced Counter charges: 2 base, +1 at E6
    let charges = if eidolon >= 6 { 3.0 } else { 2.0 };
    state.team[idx].stacks.insert(ENHANCED_REM, charges);

    // E2: ATK +30% for 2 turns
    if eidolon >= 2 {
        let old_e2 = state.team[idx].stacks.get(E2_ATK_REM).copied().unwrap_or(0.0);
        if old_e2 > 0.0 {
            state.team[idx].buffs.atk_percent -= 30.0;
        }
        state.team[idx].buffs.atk_percent += 30.0;
        state.team[idx].stacks.insert(E2_ATK_REM, 2.0);
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Ult: DMG-red +25% (2t), Enhanced Counter x{:.0}{}",
        charges,
        if eidolon >= 2 { ", ATK +30% (2t)" } else { "" }
    ));
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_enemy_action(state: &mut SimState, idx: usize, enemy_idx: usize) {
    if state.team[idx].is_downed { return; }

    // Mark the enemy that just attacked (Talent)
    state.stacks.insert(mark_key(enemy_idx), 1.0);

    // Consume an enhanced counter charge if available
    let enhanced_rem = state.team[idx].stacks.get(ENHANCED_REM).copied().unwrap_or(0.0);
    let use_enhanced = enhanced_rem > 0.0;
    if use_enhanced {
        let new_rem = enhanced_rem - 1.0;
        if new_rem <= 0.0 {
            state.team[idx].stacks.remove(ENHANCED_REM);
        } else {
            state.team[idx].stacks.insert(ENHANCED_REM, new_rem);
        }
    }

    fire_counter(state, idx, enemy_idx, use_enhanced);

    let eidolon = state.team[idx].eidolon;

    // E4: +30% DMG reduction until start of Clara's next turn
    if eidolon >= 4 && state.team[idx].stacks.get(E4_ACTIVE).copied().unwrap_or(0.0) < 1.0 {
        state.team[idx].stacks.insert(E4_ACTIVE, 1.0);
        state.team[idx].buffs.incoming_dmg_reduction += 30.0;
    }

    // E6: 50% chance counter when other ally is attacked (deterministic alternating approximation)
    if eidolon >= 6 {
        let has_ally = state.team.iter().any(|m| m.kit_id != ids::CLARA_ID && !m.is_downed);
        if has_ally {
            let flip = state.stacks.get("clara_e6_flip").copied().unwrap_or(0.0);
            if flip < 0.5 {
                state.stacks.insert("clara_e6_flip".to_string(), 1.0);
                fire_counter(state, idx, enemy_idx, false);
                state.stacks.insert(mark_key(enemy_idx), 1.0);
            } else {
                state.stacks.insert("clara_e6_flip".to_string(), 0.0);
            }
        }
    }
}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
