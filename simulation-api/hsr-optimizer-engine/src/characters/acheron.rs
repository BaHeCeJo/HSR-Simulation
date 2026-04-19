use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

fn get_sd(state: &SimState, idx: usize) -> f64 {
    state.team[idx].stacks.get("sd").copied().unwrap_or(0.0)
}

fn add_sd(state: &mut SimState, idx: usize, amount: f64) {
    let current = get_sd(state, idx);
    let overflow = ((current + amount) - 9.0).max(0.0);
    let new_sd = (current + amount).min(9.0);
    state.team[idx].stacks.insert("sd".to_string(), new_sd);
    if overflow > 0.0 {
        let qa = state.team[idx].stacks.get("qa").copied().unwrap_or(0.0);
        state.team[idx].stacks.insert("qa".to_string(), (qa + overflow).min(3.0));
    }
}

fn get_ck(state: &SimState, instance_id: &str) -> f64 {
    state.stacks.get(&format!("ck_{}", instance_id)).copied().unwrap_or(0.0)
}

fn set_ck(state: &mut SimState, instance_id: &str, value: f64) {
    state.stacks.insert(format!("ck_{}", instance_id), value.clamp(0.0, 9.0));
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    // Red Oni trace: start with 5 SD + 5 CK on a random enemy
    let eidolon = state.team[idx].eidolon;
    add_sd(state, idx, 5.0);

    if let Some(e) = state.enemies.iter().filter_map(|s| s.as_ref()).next() {
        let iid = e.instance_id.clone();
        let current = get_ck(state, &iid);
        set_ck(state, &iid, current + 5.0);
    }

    // E4: all enemies +8% vulnerability
    if eidolon >= 4 {
        for slot in state.enemies.iter_mut() {
            if let Some(e) = slot.as_mut() {
                e.vulnerability += 8.0;
            }
        }
    }

    // Minor traces
    state.team[idx].buffs.atk_percent += 28.0;
    state.team[idx].buffs.dmg_boost   +=  8.0;
    state.team[idx].buffs.crit_dmg    += 24.0;
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // E2: +1 SD at turn start
    if state.team[idx].eidolon >= 2 {
        add_sd(state, idx, 1.0);
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    let eidolon = state.team[idx].eidolon;

    // E1: +18% CRIT Rate
    if eidolon >= 1 {
        state.team[idx].buffs.crit_rate += 18.0;
    }

    // Trace: The Abyss (Nihility multiplier)
    let other_nihility = (state.nihility_count - 1).max(0);
    let effective = if eidolon >= 2 { other_nihility + 1 } else { other_nihility };
    let abyss_mult = if effective >= 2 { 1.60 } else if effective >= 1 { 1.15 } else { 1.0 };
    state.team[idx].buffs.extra_multiplier += (abyss_mult - 1.0) * 100.0;

    // Thunder Core persistent buff
    let tc_boost = state.team[idx].active_buffs.get("thunder_core")
        .map(|b| b.value)
        .unwrap_or(0.0);
    if tc_boost > 0.0 {
        state.team[idx].buffs.dmg_boost += tc_boost;
    }

    // Skill: inflict debuff → on_global_debuff fires and grants the +1 SD.
    if action.action_type == ActionType::Skill {
        action.inflicts_debuff = true;
    }

    // E6: all DMG is ult DMG + 20% RES PEN
    if eidolon >= 6 || action.action_type == ActionType::Ultimate {
        action.is_ult_dmg    = true;
        action.inflicts_debuff = true;
        if eidolon >= 6 {
            state.team[idx].buffs.res_pen += 20.0;
        }
    }
}

pub fn on_after_action(
    _state: &mut SimState,
    _idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    // Simulator does NOT reset Acheron's energy; reset SD stacks here
    let sd = get_sd(state, idx);
    let qa = state.team[idx].stacks.get("qa").copied().unwrap_or(0.0);
    state.team[idx].stacks.insert("sd".to_string(), 0.0);
    state.team[idx].stacks.insert("qa".to_string(), 0.0);

    // Talent: -20% All-Type RES during ult
    for slot in state.enemies.iter_mut() {
        if let Some(e) = slot.as_mut() { e.resistance -= 0.20; }
    }

    let member = state.team[idx].clone();
    let base_dmg_boost = member.buffs.dmg_boost;
    let mut tc_stacks = 0u32;
    let mut total_ult_dmg = 0.0f64;

    // ─── 3 Rainblades ───────────────────────────────────────────────────────────
    for _i in 0..3 {
        // Pick enemy with most CK
        let target_iid: Option<String> = {
            state.enemies.iter()
                .filter_map(|s| s.as_ref().filter(|e| e.hp > 0.0))
                .max_by(|a, b| {
                    get_ck(state, &a.instance_id)
                        .partial_cmp(&get_ck(state, &b.instance_id))
                        .unwrap()
                })
                .map(|e| e.instance_id.clone())
        };
        let target_iid = match target_iid { Some(i) => i, None => break };
        let t_idx = match state.enemies.iter().position(|s| {
            s.as_ref().map_or(false, |e| e.instance_id == target_iid && e.hp > 0.0)
        }) { Some(i) => i, None => break };

        // CK removal → Thunder Core stacks
        let ck = get_ck(state, &target_iid);
        let removed = ck.min(3.0);
        set_ck(state, &target_iid, ck - removed);
        if ck > 0.0 { tc_stacks = (tc_stacks + 1).min(3); }

        let mut rb_member = member.clone();
        rb_member.buffs.dmg_boost = base_dmg_boost + (tc_stacks as f64 * 30.0);

        // Rainblade main hit (24% ATK)
        let rb_action = ActionParams {
            action_type: ActionType::Ultimate,
            scaling_stat_id: ids::CHAR_ATK_ID.to_string(),
            multiplier: 0.24, extra_multiplier: 0.0, extra_dmg: 0.0,
            toughness_damage: 5.0, inflicts_debuff: true, is_ult_dmg: true,
        };
        let rb_dmg = state.enemies[t_idx].as_ref()
            .map(|e| damage::calculate_damage(&rb_member, e, &rb_action))
            .unwrap_or(0.0);
        if rb_dmg > 0.0 {
            if let Some(e) = state.enemies[t_idx].as_mut() { e.hp -= rb_dmg; }
            total_ult_dmg += rb_dmg;
        }

        // Rainblade AoE (15% + 15% per CK removed)
        let aoe_mult = 0.15 + removed * 0.15;
        let aoe_action = ActionParams { multiplier: aoe_mult, toughness_damage: 0.0, ..rb_action.clone() };
        for i in 0..state.enemies.len() {
            if state.enemies[i].as_ref().map_or(false, |e| e.hp > 0.0) {
                let aoe_dmg = state.enemies[i].as_ref()
                    .map(|e| damage::calculate_damage(&rb_member, e, &aoe_action))
                    .unwrap_or(0.0);
                if aoe_dmg > 0.0 {
                    if let Some(e) = state.enemies[i].as_mut() { e.hp -= aoe_dmg; }
                    total_ult_dmg += aoe_dmg;
                }
            }
        }

        // Remove dead
        for i in 0..state.enemies.len() {
            if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[i] = None;
            }
        }
    }

    // ─── Stygian Resurge AoE (120% ATK) ─────────────────────────────────────────
    let sr_action = ActionParams {
        action_type: ActionType::Ultimate,
        scaling_stat_id: ids::CHAR_ATK_ID.to_string(),
        multiplier: 1.20, extra_multiplier: 0.0, extra_dmg: 0.0,
        toughness_damage: 10.0, inflicts_debuff: true, is_ult_dmg: true,
    };
    let mut sr_member = member.clone();
    sr_member.buffs.dmg_boost = base_dmg_boost + (tc_stacks as f64 * 30.0);
    for i in 0..state.enemies.len() {
        if state.enemies[i].as_ref().map_or(false, |e| e.hp > 0.0) {
            let sr_dmg = state.enemies[i].as_ref()
                .map(|e| damage::calculate_damage(&sr_member, e, &sr_action))
                .unwrap_or(0.0);
            if sr_dmg > 0.0 {
                if let Some(e) = state.enemies[i].as_mut() {
                    e.hp -= sr_dmg;
                    // Clear all CK
                    let iid = e.instance_id.clone();
                    set_ck(state, &iid, 0.0);
                }
                total_ult_dmg += sr_dmg;
            }
        }
        if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[i] = None;
        }
    }

    // ─── Thunder Core 6 extra hits (25% ATK each, random targets) ────────────────
    let extra_action = ActionParams {
        action_type: ActionType::Ultimate,
        scaling_stat_id: ids::CHAR_ATK_ID.to_string(),
        multiplier: 0.25, extra_multiplier: 0.0, extra_dmg: 0.0,
        toughness_damage: 0.0, inflicts_debuff: false, is_ult_dmg: true,
    };
    for k in 0..6 {
        // Target first alive enemy (deterministic)
        let t_idx = state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0));
        if let Some(_t) = t_idx {
            // Cycle through enemies for spread
            let alive: Vec<usize> = state.enemies.iter().enumerate()
                .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
                .collect();
            if alive.is_empty() { break; }
            let pick = alive[k % alive.len()];
            let extra_dmg = state.enemies[pick].as_ref()
                .map(|e| damage::calculate_damage(&sr_member, e, &extra_action))
                .unwrap_or(0.0);
            if extra_dmg > 0.0 {
                if let Some(e) = state.enemies[pick].as_mut() { e.hp -= extra_dmg; }
                total_ult_dmg += extra_dmg;
            }
        }
        let _ = t_idx;
    }

    // Remove dead
    for i in 0..state.enemies.len() {
        if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[i] = None;
        }
    }

    // Revert RES
    for slot in state.enemies.iter_mut() {
        if let Some(e) = slot.as_mut() { e.resistance += 0.20; }
    }

    state.total_damage += total_ult_dmg;
    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Acheron Ult: {:.0} DMG", total_ult_dmg));

    // Thunder Core buff (3 turns, value = tc_stacks * 30%)
    if tc_stacks > 0 {
        use crate::models::StatusEffect;
        use crate::effects;
        let boost_val = tc_stacks as f64 * 30.0;
        effects::apply_member_buff(&mut state.team[idx], "thunder_core", StatusEffect {
            duration: 3, value: boost_val, stat: None, effects: vec![],
        });
    }

    // QA → SD refund
    if qa > 0.0 {
        add_sd(state, idx, qa);
    }
    let _ = sd;
}

pub fn on_global_debuff(state: &mut SimState, idx: usize, _source_idx: usize, enemy_idx: usize) {
    // Gain 1 SD per action (dedup by current_action_id)
    let action_id = state.current_action_id;
    let last = state.team[idx].stacks.get("acheron_last_debuff_action").copied().unwrap_or(0.0);
    if last as u64 == action_id { return; }
    state.team[idx].stacks.insert("acheron_last_debuff_action".to_string(), action_id as f64);

    add_sd(state, idx, 1.0);

    // Add CK to affected enemy
    if let Some(enemy) = state.enemies[enemy_idx].as_ref() {
        let iid = enemy.instance_id.clone();
        let ck = get_ck(state, &iid);
        set_ck(state, &iid, ck + 1.0);
    }
}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
