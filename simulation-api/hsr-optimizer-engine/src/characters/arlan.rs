use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy        = 110.0;
    state.team[idx].buffs.atk_percent += 28.0;  // minor trace: ATK +28%
    state.team[idx].buffs.hp_percent  += 10.0;  // minor trace: HP +10%
    state.team[idx].buffs.effect_res  += 18.0;  // minor trace: Effect RES +18%

    // A4: +50% Effect RES against DoT debuffs specifically
    state.team[idx].stacks.insert("arlan_dot_effect_res", 50.0);

    // E4: survive one killing blow
    if state.team[idx].eidolon >= 4 {
        state.team[idx].stacks.insert("arlan_e4_active", 1.0);
        state.team[idx].stacks.insert("arlan_e4_turns_left", 2.0);
    }

    // A6: if starting HP ≤ 50%, nullify first incoming hit
    let hp_pct = state.team[idx].hp / state.team[idx].max_hp;
    if hp_pct <= 0.5 {
        state.team[idx].stacks.insert("arlan_a6_active", 1.0);
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // E4: decrement survival counter
    if state.team[idx].eidolon >= 4 {
        let left = state.team[idx].stacks.get("arlan_e4_turns_left").copied().unwrap_or(0.0);
        if left > 0.0 {
            let new_left = left - 1.0;
            state.team[idx].stacks.insert("arlan_e4_turns_left", new_left);
            if new_left <= 0.0 {
                state.team[idx].stacks.remove("arlan_e4_active");
            }
        }
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    // Skill: consume 15% Max HP
    if action.action_type == ActionType::Skill {
        let max_hp = state.team[idx].max_hp;
        let cost = max_hp * 0.15;
        state.team[idx].hp = (state.team[idx].hp - cost).max(1.0);
    }

    // Talent: +0.72% DMG per 1% missing HP (max 72%)
    let hp_pct = if state.team[idx].max_hp > 0.0 {
        state.team[idx].hp / state.team[idx].max_hp
    } else { 1.0 };
    let talent_boost = (1.0 - hp_pct) * 72.0;
    state.team[idx].buffs.dmg_boost += talent_boost;

    // E1: Skill +10% DMG when HP ≤ 50%
    if state.team[idx].eidolon >= 1 && action.action_type == ActionType::Skill && hp_pct <= 0.5 {
        state.team[idx].buffs.dmg_boost += 10.0;
    }

    // E6: Ult +20% DMG when HP ≤ 50%
    if state.team[idx].eidolon >= 6 && action.action_type == ActionType::Ultimate && hp_pct <= 0.5 {
        state.team[idx].buffs.dmg_boost += 20.0;
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // E2: remove first debuff after Skill
    if state.team[idx].eidolon >= 2 && action.action_type == ActionType::Skill {
        let key = state.team[idx].active_debuffs.keys().next().cloned();
        if let Some(k) = key {
            state.team[idx].active_debuffs.remove(&k);
        }
    }

    // A2: restore 20% Max HP on kill if Arlan HP ≤ 30%
    if let Some(t) = target_idx {
        let enemy_dead = state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0);
        if enemy_dead {
            let hp_pct = state.team[idx].hp / state.team[idx].max_hp;
            if hp_pct <= 0.30 {
                let restore = state.team[idx].max_hp * 0.20;
                let max_hp = state.team[idx].max_hp;
                state.team[idx].hp = (state.team[idx].hp + restore).min(max_hp);
            }
        }
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    // E2: remove first debuff on ult
    if state.team[idx].eidolon >= 2 {
        let key = state.team[idx].active_debuffs.keys().next().cloned();
        if let Some(k) = key {
            state.team[idx].active_debuffs.remove(&k);
        }
    }

    let hp_pct = if state.team[idx].max_hp > 0.0 {
        state.team[idx].hp / state.team[idx].max_hp
    } else { 1.0 };
    let eidolon = state.team[idx].eidolon;
    let adjacent_mult = if eidolon >= 6 && hp_pct <= 0.5 { 3.2 } else { 1.6 };

    // Pick main target (first alive enemy)
    let t_idx = match state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0)) {
        Some(i) => i,
        None => return,
    };

    let member = state.team[idx].clone();
    let main_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       3.2,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    // Main hit
    let main_dmg = state.enemies[t_idx].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &main_action))
        .unwrap_or(0.0);
    if main_dmg > 0.0 {
        if let Some(e) = state.enemies[t_idx].as_mut() { e.hp -= main_dmg; }
        state.total_damage += main_dmg;
    }
    if state.enemies[t_idx].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[t_idx] = None;
    }

    // Adjacent hits
    let adj_action = ActionParams { multiplier: adjacent_mult, ..main_action.clone() };
    let left  = if t_idx > 0 { Some(t_idx - 1) } else { None };
    let right = if t_idx + 1 < state.enemies.len() { Some(t_idx + 1) } else { None };

    for adj in [left, right].iter().flatten() {
        let adj_alive = state.enemies[*adj].as_ref().map_or(false, |e| e.hp > 0.0);
        if !adj_alive { continue; }
        let adj_dmg = state.enemies[*adj].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &adj_action))
            .unwrap_or(0.0);
        if adj_dmg > 0.0 {
            if let Some(e) = state.enemies[*adj].as_mut() { e.hp -= adj_dmg; }
            state.total_damage += adj_dmg;
        }
        if state.enemies[*adj].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[*adj] = None;
        }
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Frenzied Punishment: {:.0} DMG", main_dmg));
}

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
