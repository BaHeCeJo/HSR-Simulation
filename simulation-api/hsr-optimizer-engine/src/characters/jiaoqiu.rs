use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

const ZONE_KEY: &str = "jiaoqiu_zone_turns";
const ZONE_TRIGGERS: &str = "jiaoqiu_zone_triggers";

fn ashen_key(instance_id: &str) -> String {
    format!("ashen_roast_{}", instance_id)
}

fn get_stacks(state: &SimState, instance_id: &str) -> f64 {
    state.stacks.get(&ashen_key(instance_id)).copied().unwrap_or(0.0)
}

fn vuln_for_stacks(stacks: f64) -> f64 {
    if stacks <= 0.0 { 0.0 } else { 15.0 + (stacks - 1.0) * 5.0 }
}

fn update_vuln_buff(state: &mut SimState, enemy_idx: usize) {
    let (instance_id, stacks) = {
        let e = match state.enemies[enemy_idx].as_ref() { Some(e) => e, None => return };
        (e.instance_id.clone(), get_stacks(state, &e.instance_id))
    };
    let vuln = vuln_for_stacks(stacks);
    if let Some(e) = state.enemies[enemy_idx].as_mut() {
        if vuln > 0.0 {
            effects::apply_enemy_buff(e, "ashen_roast_vuln", StatusEffect {
                duration: 999,
                value:    vuln,
                stat:     Some("Vulnerability".to_string()),
                effects:  vec![],
            });
        } else {
            e.active_buffs.remove("ashen_roast_vuln");
        }
    }
    let _ = instance_id;
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy        = 100.0;
    state.team[idx].buffs.atk_percent += 28.0;  // minor trace: ATK +28%
    state.team[idx].buffs.dmg_boost   += 14.4;  // minor trace: Fire DMG +14.4%
    state.team[idx].buffs.crit_rate   += 12.0;  // minor trace: CRIT Rate +12%
    // A2: start with 15 energy
    state.team[idx].energy = 15.0;
}

pub fn on_turn_start(state: &mut SimState, _idx: usize) {
    // Decrement zone duration
    let turns = state.stacks.get(ZONE_KEY).copied().unwrap_or(0.0);
    if turns > 0.0 {
        let new_turns = turns - 1.0;
        state.stacks.insert(ZONE_KEY.to_string(), new_turns);
        if new_turns <= 0.0 {
            state.stacks.insert(ZONE_TRIGGERS.to_string(), 0.0);
            // Remove zone vulnerability from all enemies
            for slot in state.enemies.iter_mut() {
                if let Some(e) = slot.as_mut() {
                    e.active_buffs.remove("jiaoqiu_zone_vuln");
                }
            }
        }
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    action.inflicts_debuff = true;

    // E1: +40% DMG boost
    if state.team[idx].eidolon >= 1 {
        state.team[idx].buffs.dmg_boost += 40.0;
    }

    // E6: +3% RES PEN per Ashen Roast stack on target (max 9 stacks = +27%)
    if state.team[idx].eidolon >= 6 {
        if let Some(t) = target_idx {
            if let Some(enemy) = state.enemies[t].as_ref() {
                let stacks = get_stacks(state, &enemy.instance_id);
                state.team[idx].buffs.res_pen += stacks * 3.0;
            }
        }
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    if !matches!(
        action.action_type,
        ActionType::Basic | ActionType::Skill | ActionType::Ultimate
    ) {
        return;
    }

    let t = match target_idx { Some(t) => t, None => return };
    let enemy = match state.enemies[t].as_ref() { Some(e) => e, None => return };
    let instance_id = enemy.instance_id.clone();
    let eidolon = state.team[idx].eidolon;
    let max_stacks = if eidolon >= 6 { 9.0 } else { 5.0 };
    let added = if eidolon >= 1 { 2.0 } else { 1.0 };

    let current = get_stacks(state, &instance_id);
    let new_stacks = (current + added).min(max_stacks);
    state.stacks.insert(ashen_key(&instance_id), new_stacks);

    update_vuln_buff(state, t);
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    // Signal ult handled
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    // Energy reset (simulator already set to 0; grant 5 back)
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;
    let max_stacks = if eidolon >= 6 { 9.0 } else { 5.0 };

    // Equalize all enemy stacks to max on field (min 1)
    let max_on_field = state.enemies.iter()
        .filter_map(|slot| slot.as_ref())
        .map(|e| get_stacks(state, &e.instance_id))
        .fold(0.0_f64, f64::max);
    let eq_stacks = max_on_field.max(1.0).min(max_stacks);

    let instance_ids: Vec<String> = state.enemies.iter()
        .filter_map(|slot| slot.as_ref().map(|e| e.instance_id.clone()))
        .collect();
    for iid in &instance_ids {
        state.stacks.insert(ashen_key(iid), eq_stacks);
    }

    // Update vuln buffs and apply zone vulnerability
    let zone_vuln_val = 15.0;
    let enemy_count = state.enemies.iter().filter(|s| s.is_some()).count();
    for i in 0..state.enemies.len() {
        if state.enemies[i].is_none() { continue; }
        update_vuln_buff(state, i);
        if let Some(e) = state.enemies[i].as_mut() {
            effects::apply_enemy_buff(e, "jiaoqiu_zone_vuln", StatusEffect {
                duration: 999, value: zone_vuln_val,
                stat: Some("Vulnerability".to_string()), effects: vec![],
            });
        }
    }

    // Create zone: 3 turns, 6 triggers
    state.stacks.insert(ZONE_KEY.to_string(), 3.0);
    state.stacks.insert(ZONE_TRIGGERS.to_string(), 6.0);

    // Deal ult damage (100% ATK AoE) manually
    let member = state.team[idx].clone();
    let ult_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       1.0,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  true,
        is_ult_dmg:       true,
    };

    let mut total_ult_dmg = 0.0f64;
    for i in 0..state.enemies.len() {
        if state.enemies[i].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }
        let dmg = state.enemies[i].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &ult_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[i].as_mut() { e.hp -= dmg; }
            total_ult_dmg += dmg;
        }
        if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[i] = None;
        }
    }
    state.total_damage += total_ult_dmg;
    let name = member.name.clone();
    state.add_log(&name, format!("Jiaoqiu Ult: {:.0} DMG", total_ult_dmg));
    let _ = enemy_count;
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(state: &mut SimState, idx: usize, enemy_idx: usize) {
    let (instance_id, hp) = {
        let e = match state.enemies[enemy_idx].as_ref() { Some(e) => e, None => return };
        (e.instance_id.clone(), e.hp)
    };
    if hp <= 0.0 { return; }

    let stacks = get_stacks(state, &instance_id);
    if stacks <= 0.0 { return; }

    let eidolon = state.team[idx].eidolon;
    let mult = if eidolon >= 2 { 1.8 + 3.0 } else { 1.8 };

    let member = state.team[idx].clone();
    let dot_action = ActionParams {
        action_type:      ActionType::TalentProc,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       mult,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 0.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let dmg = state.enemies[enemy_idx].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &dot_action))
        .unwrap_or(0.0);

    if dmg > 0.0 {
        if let Some(e) = state.enemies[enemy_idx].as_mut() {
            e.hp -= dmg;
            if e.hp <= 0.0 { state.enemies[enemy_idx] = None; }
        }
        state.total_damage += dmg;
        state.add_log(&member.name, format!("Ashen Roast DoT: {:.0} DMG", dmg));
    }
}

pub fn on_enemy_action(state: &mut SimState, idx: usize, enemy_idx: usize) {
    let triggers = state.stacks.get(ZONE_TRIGGERS).copied().unwrap_or(0.0);
    if triggers <= 0.0 { return; }
    let zone_turns = state.stacks.get(ZONE_KEY).copied().unwrap_or(0.0);
    if zone_turns <= 0.0 { return; }

    let eidolon = state.team[idx].eidolon;
    let max_stacks = if eidolon >= 6 { 9.0 } else { 5.0 };
    let instance_id = match state.enemies[enemy_idx].as_ref() {
        Some(e) => e.instance_id.clone(),
        None => return,
    };

    let current = get_stacks(state, &instance_id);
    if current < max_stacks {
        state.stacks.insert(ashen_key(&instance_id), current + 1.0);
        state.stacks.insert(ZONE_TRIGGERS.to_string(), triggers - 1.0);
        update_vuln_buff(state, enemy_idx);
    }
}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
