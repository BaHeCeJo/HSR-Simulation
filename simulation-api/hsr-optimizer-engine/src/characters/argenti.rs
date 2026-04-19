use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

const APO_KEY: &str    = "argenti_apotheosis";
const ENERGY_KEY: &str = "argenti_energy";

fn get_apo(state: &SimState) -> f64 {
    state.stacks.get(APO_KEY).copied().unwrap_or(0.0)
}

fn add_apo(state: &mut SimState, eidolon: i32, count: f64) {
    let max = if eidolon >= 4 { 12.0 } else { 10.0 };
    let current = get_apo(state);
    state.stacks.insert(APO_KEY.to_string(), (current + count).min(max));
}

fn add_argenti_energy(state: &mut SimState, idx: usize, amount: f64) {
    let current = state.stacks.get(ENERGY_KEY).copied().unwrap_or(0.0);
    let new_val = current + amount;
    state.stacks.insert(ENERGY_KEY.to_string(), new_val);
    // Signal ult ready via _ult_ready flag when threshold reached
    let prefer_90 = state.stacks.get("argenti_prefer_90_ult").copied().unwrap_or(0.0) >= 1.0;
    let threshold = if prefer_90 { 90.0 } else { 180.0 };
    if new_val >= threshold {
        state.team[idx].stacks.insert("_ult_ready".to_string(), 1.0);
    }
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    // Use _ult_ready flag system; set max_energy high so auto-ult doesn't fire
    state.team[idx].max_energy = f64::MAX;
    state.team[idx].buffs.atk_percent += 28.0;  // minor trace: ATK +28%
    state.team[idx].buffs.hp_percent  += 10.0;  // minor trace: HP +10%
    state.team[idx].buffs.dmg_boost   += 14.4;  // minor trace: Physical DMG +14.4%

    state.stacks.insert(APO_KEY.to_string(), 0.0);
    state.stacks.insert(ENERGY_KEY.to_string(), 0.0);

    // E4: start with 2 Apotheosis stacks
    if state.team[idx].eidolon >= 4 {
        add_apo(state, state.team[idx].eidolon, 2.0);
    }

    // A4: +2 energy per enemy at battle start
    let enemy_count = state.enemies.iter().filter(|s| s.is_some()).count() as f64;
    add_argenti_energy(state, idx, enemy_count * 2.0);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // A2: +1 Apotheosis per turn
    let eidolon = state.team[idx].eidolon;
    add_apo(state, eidolon, 1.0);
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    let apo = get_apo(state);
    let eidolon = state.team[idx].eidolon;

    // Talent: +2.5% CRIT Rate per Apotheosis stack
    state.team[idx].buffs.crit_rate += apo * 2.5;

    // E1: +4% CRIT DMG per stack
    if eidolon >= 1 {
        state.team[idx].buffs.crit_dmg += apo * 4.0;
    }

    // E2: 3+ alive enemies → +40% ATK on ult
    if eidolon >= 2 && action.action_type == ActionType::Ultimate {
        let alive = state.enemies.iter().filter(|s| s.as_ref().map_or(false, |e| e.hp > 0.0)).count();
        if alive >= 3 {
            state.team[idx].buffs.atk_percent += 40.0;
        }
    }

    // E6: ult +30% DEF ignore
    if eidolon >= 6 && action.action_type == ActionType::Ultimate {
        state.team[idx].buffs.def_ignore += 30.0;
    }

    // A6: +15% DMG boost if target HP ≤ 50%
    if let Some(t) = target_idx {
        if let Some(enemy) = state.enemies[t].as_ref() {
            if enemy.max_hp > 0.0 && enemy.hp / enemy.max_hp <= 0.5 {
                state.team[idx].buffs.dmg_boost += 15.0;
            }
        }
    }

    // Cache alive count for on_after_action
    let alive_count = state.enemies.iter().filter(|s| s.as_ref().map_or(false, |e| e.hp > 0.0)).count();
    state.stacks.insert("argenti_pre_alive".to_string(), alive_count as f64);
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    _target_idx: Option<usize>,
) {
    if action.action_type == ActionType::Ultimate { return; }

    let eidolon = state.team[idx].eidolon;
    let pre_alive = state.stacks.get("argenti_pre_alive").copied().unwrap_or(1.0) as usize;
    let enemies_hit = if action.action_type == ActionType::Basic { 1 } else { pre_alive.max(1) };

    // Talent: +3 energy + 1 apo per enemy hit
    add_apo(state, eidolon, enemies_hit as f64);
    let std_energy = if action.action_type == ActionType::Basic { 20.0 } else { 30.0 };
    let talent_energy = (enemies_hit as f64) * 3.0;
    add_argenti_energy(state, idx, std_energy + talent_energy);

    // Prevent the simulator from also adding energy (we manage it manually)
    state.team[idx].energy = 0.0;
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].stacks.remove("_ult_ready");

    let actual_energy = state.stacks.get(ENERGY_KEY).copied().unwrap_or(0.0);
    let use_180 = actual_energy >= 180.0;
    state.stacks.insert(ENERGY_KEY.to_string(), 0.0);

    let eidolon = state.team[idx].eidolon;
    let member = state.team[idx].clone();
    let base_dmg_boost = member.buffs.dmg_boost;
    let mut total_dmg = 0.0f64;

    let ult_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       0.0, // set per hit
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };

    let (main_mult, extra_count) = if use_180 { (2.8, 6) } else { (1.6, 0) };

    // AoE main hits
    let alive_indices: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();

    for &i in &alive_indices {
        let a6_bonus = state.enemies[i].as_ref()
            .map(|e| if e.max_hp > 0.0 && e.hp / e.max_hp <= 0.5 { 15.0 } else { 0.0 })
            .unwrap_or(0.0);
        let mut hit_member = member.clone();
        hit_member.buffs.dmg_boost = base_dmg_boost + a6_bonus;
        let action = ActionParams { multiplier: main_mult, ..ult_action.clone() };
        let dmg = state.enemies[i].as_ref()
            .map(|e| damage::calculate_damage(&hit_member, e, &action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[i].as_mut() { e.hp -= dmg; }
            total_dmg += dmg;
        }
        if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[i] = None;
        }
    }

    // 6 extra random hits (180-energy ult)
    let mut extra_landed = 0;
    for k in 0..extra_count {
        let alive: Vec<usize> = state.enemies.iter().enumerate()
            .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
            .collect();
        if alive.is_empty() { break; }
        let pick = alive[k % alive.len()];
        let a6_bonus = state.enemies[pick].as_ref()
            .map(|e| if e.max_hp > 0.0 && e.hp / e.max_hp <= 0.5 { 15.0 } else { 0.0 })
            .unwrap_or(0.0);
        let mut hit_member = member.clone();
        hit_member.buffs.dmg_boost = base_dmg_boost + a6_bonus;
        let action = ActionParams { multiplier: 0.95, toughness_damage: 0.0, ..ult_action.clone() };
        let dmg = state.enemies[pick].as_ref()
            .map(|e| damage::calculate_damage(&hit_member, e, &action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[pick].as_mut() { e.hp -= dmg; }
            total_dmg += dmg;
        }
        if state.enemies[pick].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[pick] = None;
        }
        extra_landed += 1;
    }

    state.total_damage += total_dmg;

    // Post-ult: apotheosis + energy
    let total_hits = alive_indices.len() + extra_landed;
    add_apo(state, eidolon, total_hits as f64);
    let post_energy = 5.0 + (total_hits as f64) * 3.0;
    add_argenti_energy(state, idx, post_energy);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Argenti Ult ({}): {:.0} DMG", if use_180 { "180" } else { "90" }, total_dmg));
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
