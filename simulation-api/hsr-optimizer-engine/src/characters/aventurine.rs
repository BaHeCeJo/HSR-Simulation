use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

// LC DEF UUID from TypeScript source (different from ids::LC_DEF_ID placeholder)
const LC_DEF_UUID: &str     = "52566b38-915c-4220-ab0e-61438225704b";

const BB_KEY: &str          = "aventurine_bb";
const E4_DEF_TURNS: &str    = "aventurine_e4_def_turns";

fn get_def(member: &crate::models::TeamMember) -> f64 {
    let base = member.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(0.0)
        + member.lightcone.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(0.0);
    base * (1.0 + member.buffs.def_percent / 100.0)
}

fn skill_shield(member: &crate::models::TeamMember) -> f64 {
    0.24 * get_def(member) + 320.0
}

fn a6_shield(member: &crate::models::TeamMember) -> f64 {
    0.072 * get_def(member) + 96.0
}

fn apply_shields_to_all(state: &mut SimState, av_idx: usize, shield_val: f64) {
    let cap = 2.0 * skill_shield(&state.team[av_idx]);
    for i in 0..state.team.len() {
        if !state.team[i].is_downed {
            state.team[i].shield = (state.team[i].shield + shield_val).min(cap);
        }
    }
}

fn add_bb(state: &mut SimState, idx: usize, amount: f64) {
    let bb = (state.stacks.get(BB_KEY).copied().unwrap_or(0.0) + amount).min(10.0);
    state.stacks.insert(BB_KEY.to_string(), bb);
    if bb >= 7.0 {
        fire_talent_fup(state, idx);
    }
}

fn fire_talent_fup(state: &mut SimState, idx: usize) {
    let bb = state.stacks.get(BB_KEY).copied().unwrap_or(0.0);
    if bb < 7.0 { return; }
    state.stacks.insert(BB_KEY.to_string(), bb - 7.0);

    let eidolon = state.team[idx].eidolon;

    // E4: +40% DEF for 2 turns before FUP
    if eidolon >= 4 {
        state.team[idx].buffs.def_percent += 40.0;
        state.stacks.insert(E4_DEF_TURNS.to_string(), 2.0);
    }

    let hits = if eidolon >= 4 { 10 } else { 7 };

    // Relic: reset Ashblazing Grand Duke stacks at start of new FUA sequence.
    crate::relics::on_follow_up_start(&mut state.team, idx);

    let mut fup_member = state.team[idx].clone();

    // E6: +50% DMG per shielded ally (max +150%)
    if eidolon >= 6 {
        let shielded = state.team.iter().filter(|m| !m.is_downed && m.shield > 0.0).count();
        fup_member.buffs.dmg_boost += (shielded.min(3) as f64) * 50.0;
    }

    let fup_action = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_DEF_ID.to_string(),
        multiplier:       0.25,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 3.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let mut total = 0.0f64;
    for k in 0..hits {
        let alive: Vec<usize> = state.enemies.iter().enumerate()
            .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
            .collect();
        if alive.is_empty() { break; }
        let pick = alive[k % alive.len()];
        let dmg = state.enemies[pick].as_ref()
            .map(|e| damage::calculate_damage(&fup_member, e, &fup_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[pick].as_mut() { e.hp -= dmg; }
            total += dmg;
        }
        if state.enemies[pick].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[pick] = None;
        }
        // Relic: increment Ashblazing Grand Duke stack per hit.
        crate::relics::on_follow_up_hit(&mut state.team, idx);
    }
    state.total_damage += total;

    // Relic: set Wind-Soaring Valorous post-FUA Ult DMG window.
    crate::relics::on_follow_up_end(&mut state.team, idx);

    // A6: mini-shield to all + extra to lowest-shield ally
    let a6_val = a6_shield(&state.team[idx]);
    let cap = 2.0 * skill_shield(&state.team[idx]);
    for i in 0..state.team.len() {
        if !state.team[i].is_downed {
            state.team[i].shield = (state.team[i].shield + a6_val).min(cap);
        }
    }
    if let Some(lowest) = (0..state.team.len())
        .filter(|&i| !state.team[i].is_downed)
        .min_by(|&a, &b| state.team[a].shield.partial_cmp(&state.team[b].shield)
            .unwrap_or(std::cmp::Ordering::Equal))
    {
        state.team[lowest].shield = (state.team[lowest].shield + a6_val).min(cap);
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Shot Loaded Right FUP: {} hits, {:.0} DMG", hits, total));
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 110.0;
    state.team[idx].buffs.def_percent += 35.0; // minor trace: DEF +35%
    state.team[idx].buffs.dmg_boost   += 14.4; // minor trace: Imaginary DMG +14.4%
    state.team[idx].buffs.effect_res  += 10.0; // minor trace: Effect RES +10%

    // Fold LC DEF into base_stats[CHAR_DEF_ID] for formula compatibility
    let lc_def = state.team[idx].lightcone.base_stats.get(LC_DEF_UUID).copied().unwrap_or(0.0);
    if lc_def > 0.0 {
        let entry = state.team[idx].base_stats
            .entry(ids::CHAR_DEF_ID.to_string()).or_insert(0.0);
        *entry += lc_def;
    }

    state.stacks.insert(BB_KEY.to_string(), 0.0);

    // A2: +2% CRIT Rate per 100 DEF above 1600 (max +48%)
    let total_def = get_def(&state.team[idx]);
    let cr_gain = ((total_def - 1600.0).max(0.0) / 100.0).floor() * 2.0;
    state.team[idx].buffs.crit_rate += cr_gain.min(48.0);

    // A4: all allies get skill-level shield at battle start
    let shield_val = skill_shield(&state.team[idx]);
    apply_shields_to_all(state, idx, shield_val);

    // Talent: while Aventurine has a Shield, all allies gain Effect RES
    // (A4 just gave everyone shields, so this fires immediately at battle start)
    if state.team[idx].shield > 0.0 {
        let talent_level = state.team[idx].ability_levels.talent as f64;
        let eff_res_gain = if talent_level >= 12.0 { 55.0 } else { 25.0 + (talent_level - 1.0) * 2.5 };
        for i in 0..state.team.len() {
            state.team[i].buffs.effect_res += eff_res_gain;
        }
    }

    // E1: all shielded allies +20% CRIT DMG
    if state.team[idx].eidolon >= 1 {
        for i in 0..state.team.len() {
            state.team[i].buffs.crit_dmg += 20.0;
        }
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // E4: decrement DEF boost duration
    let turns = state.stacks.get(E4_DEF_TURNS).copied().unwrap_or(0.0);
    if turns > 0.0 {
        let new = turns - 1.0;
        state.stacks.insert(E4_DEF_TURNS.to_string(), new);
        if new <= 0.0 && state.team[idx].eidolon >= 4 {
            state.team[idx].buffs.def_percent -= 40.0;
        }
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    // All of Aventurine's attacks scale off DEF, not ATK
    if matches!(action.action_type, ActionType::Basic | ActionType::Skill | ActionType::FollowUp) {
        action.scaling_stat_id = ids::CHAR_DEF_ID.to_string();
    }

    // Unnerved: +15% CRIT DMG when hitting an Unnerved enemy
    if let Some(t) = target_idx {
        if state.enemies[t].as_ref().map_or(false, |e| e.active_debuffs.contains_key("aventurine_unnerved")) {
            state.team[idx].buffs.crit_dmg += 15.0;
        }
    }

    // E6: +50% DMG per shielded ally (max +150%)
    if state.team[idx].eidolon >= 6 {
        let shielded = state.team.iter().filter(|m| !m.is_downed && m.shield > 0.0).count();
        state.team[idx].buffs.dmg_boost += (shielded.min(3) as f64) * 50.0;
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            // E2: All-Type RES -12% for 3 turns on hit target
            if state.team[idx].eidolon >= 2 {
                if let Some(t) = target_idx {
                    let ehr = state.team[idx].base_stats.get(crate::ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
                            + state.team[idx].buffs.effect_hit_rate;
                    let already = state.enemies[t].as_ref()
                        .map_or(false, |e| e.active_debuffs.contains_key("aventurine_e2_res"));
                    let landed = if !already {
                        if let Some(e) = state.enemies[t].as_mut() {
                            effects::try_apply_enemy_debuff(ehr, e, "aventurine_e2_res", StatusEffect {
                                duration: 3, value: 12.0,
                                stat: Some("All-Type RES".to_string()), effects: vec![],
                            }, 1.0)
                        } else { false }
                    } else { false };
                    if landed {
                        if let Some(e) = state.enemies[t].as_mut() {
                            e.resistance = (e.resistance - 0.12).max(-1.0);
                            for res in e.elemental_res.values_mut() {
                                *res = (*res - 0.12).max(-1.0);
                            }
                        }
                    }
                }
            }
        }
        ActionType::Skill => {
            // Apply skill shields to all allies
            let shield_val = skill_shield(&state.team[idx]);
            apply_shields_to_all(state, idx, shield_val);
        }
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;

    // Deal 270% DEF damage to first alive enemy
    let alive_idx = state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0));
    if let Some(t) = alive_idx {
        let mut ult_member = state.team[idx].clone();
        if eidolon >= 6 {
            let shielded = state.team.iter().filter(|m| !m.is_downed && m.shield > 0.0).count();
            ult_member.buffs.dmg_boost += (shielded.min(3) as f64) * 50.0;
        }
        let ult_action = ActionParams {
            action_type:      ActionType::Ultimate,
            scaling_stat_id:  ids::CHAR_DEF_ID.to_string(),
            multiplier:       2.7,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 30.0,
            inflicts_debuff:  true,
            is_ult_dmg:       true,
        };
        let dmg = state.enemies[t].as_ref()
            .map(|e| damage::calculate_damage(&ult_member, e, &ult_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
            state.total_damage += dmg;
        }
        if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[t] = None;
        }

        // Apply Unnerved debuff to enemy
        let ehr = state.team[idx].base_stats.get(crate::ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
                + state.team[idx].buffs.effect_hit_rate;
        if let Some(e) = state.enemies[t].as_mut() {
            effects::try_apply_enemy_debuff(ehr, e, "aventurine_unnerved", StatusEffect {
                duration: 3, value: 15.0,
                stat: Some("Unnerved".to_string()), effects: vec![],
            }, 1.0);
        }

        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Roulette Shark: {:.0} DMG (270% DEF)", dmg));
    }

    // Add BB (4 deterministic, average of 1d7)
    add_bb(state, idx, 4.0);

    // E1: grant all allies skill shield after ult
    if eidolon >= 1 {
        let shield_val = skill_shield(&state.team[idx]);
        apply_shields_to_all(state, idx, shield_val);
    }
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_enemy_action(state: &mut SimState, idx: usize, _enemy_idx: usize) {
    if state.team[idx].is_downed { return; }
    let av_kit = state.team[idx].kit_id.clone();

    // +1 BB if any non-Aventurine shielded ally is attacked
    let any_shielded = state.team.iter()
        .any(|m| !m.is_downed && m.shield > 0.0 && m.kit_id != av_kit);
    if any_shielded {
        add_bb(state, idx, 1.0);
    }

    // +1 BB if Aventurine himself is shielded
    if state.team[idx].shield > 0.0 {
        add_bb(state, idx, 1.0);
    }
}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
