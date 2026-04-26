use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, ActorEntry, SimState};

// ─── Per-member stack keys (static, go in TeamMember.stacks) ──────────────────
const ADVANCE_PCT: &str = "_action_advance_pct"; // Talent: 30% forward advance after Basic

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn a4_rem_key(i: usize)   -> String { format!("bronya_a4_{i}") }
fn tech_rem_key(i: usize) -> String { format!("bronya_tech_{i}") }
fn ult_rem_key(i: usize)  -> String { format!("bronya_ult_{i}") }

fn remove_ult_buff(state: &mut SimState, i: usize, cd_bonus: f64) {
    state.team[i].buffs.atk_percent -= 55.0;
    state.team[i].buffs.crit_dmg   -= cd_bonus;
    state.stacks.remove(&ult_rem_key(i));
}

fn apply_ult_buff(state: &mut SimState, i: usize, cd_bonus: f64) {
    state.team[i].buffs.atk_percent += 55.0;
    state.team[i].buffs.crit_dmg   += cd_bonus;
    state.stacks.insert(ult_rem_key(i), 2.0);
}

/// Decrement one of Bronya's timed ally buffs (A4 DEF, Technique ATK, or Ult).
/// Removes the buff from the ally's stats when the counter hits 0.
fn tick_a4_buff(state: &mut SimState, i: usize) {
    let key = a4_rem_key(i);
    let rem = state.stacks.get(&key).copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }
    if rem <= 1.0 {
        state.stacks.remove(&key);
        state.team[i].buffs.def_percent -= 20.0;
    } else {
        state.stacks.insert(key, rem - 1.0);
    }
}

fn tick_tech_buff(state: &mut SimState, i: usize) {
    let key = tech_rem_key(i);
    let rem = state.stacks.get(&key).copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }
    if rem <= 1.0 {
        state.stacks.remove(&key);
        state.team[i].buffs.atk_percent -= 15.0;
    } else {
        state.stacks.insert(key, rem - 1.0);
    }
}

fn tick_ult_buff(state: &mut SimState, i: usize) {
    let key = ult_rem_key(i);
    let rem = state.stacks.get(&key).copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }
    let cd_bonus = state.stacks.get("bronya_ult_cd_bonus").copied().unwrap_or(0.0);
    if rem <= 1.0 {
        remove_ult_buff(state, i, cd_bonus);
    } else {
        state.stacks.insert(key, rem - 1.0);
    }
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy        = 120.0;
    state.team[idx].buffs.dmg_boost  += 22.4; // minor trace: Wind DMG +22.4%
    state.team[idx].buffs.crit_dmg   += 24.0; // minor trace: CRIT DMG +24%
    state.team[idx].buffs.effect_res += 10.0; // minor trace: Effect RES +10%

    state.stacks.insert("bronya_skill_target".to_string(),    -1.0);
    state.stacks.insert("bronya_skill_remaining".to_string(),  0.0);
    state.stacks.insert("bronya_ult_cd_bonus".to_string(),     0.0);
    state.stacks.insert("bronya_e1_sp_acc".to_string(),        0.0);
    state.stacks.insert("bronya_e4_used".to_string(),          0.0);
    state.stacks.insert("bronya_e2_target".to_string(),       -1.0);
    state.stacks.insert("bronya_e2_spd_inc".to_string(),       0.0);

    // Technique: all allies +15% ATK for 2 turns at battle start
    // A4: all allies +20% DEF for 2 turns at battle start
    // A6: all allies +10% DMG while Bronya is on field (permanent)
    for i in 0..state.team.len() {
        if !state.team[i].is_downed {
            state.team[i].buffs.atk_percent += 15.0;
            state.stacks.insert(tech_rem_key(i), 2.0);

            state.team[i].buffs.def_percent += 20.0;
            state.stacks.insert(a4_rem_key(i), 2.0);

            state.team[i].buffs.dmg_boost += 10.0;
        }
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // Clear talent advance (was set last basic ATK, used to compute AV)
    state.team[idx].stacks.remove(ADVANCE_PCT);

    // E4 resets each Bronya turn
    state.stacks.insert("bronya_e4_used".to_string(), 0.0);

    // Tick Bronya's own timed buffs (other allies tick in on_ally_action)
    tick_a4_buff(state, idx);
    tick_tech_buff(state, idx);
    tick_ult_buff(state, idx);
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            // A2: CRIT Rate for Basic ATK = 100% (clamped to 1.0 in damage formula)
            state.team[idx].buffs.crit_rate += 100.0;
        }
        ActionType::Skill => {
            // Bronya's skill deals no damage — zero out the multiplier
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
    match action.action_type {
        ActionType::Basic => {
            // Talent: Bronya's next action is Advanced Forward by 30%.
            // effective_spd() in simulator.rs reads this and applies spd / (1 - 0.30).
            state.team[idx].stacks.insert(ADVANCE_PCT, 30.0);
        }

        ActionType::Skill => {
            let eidolon   = state.team[idx].eidolon;
            let skill_dur = if eidolon >= 6 { 2.0 } else { 1.0 };

            // Remove existing skill buff from previous target if still active
            let old_t   = state.stacks.get("bronya_skill_target").copied().unwrap_or(-1.0);
            let old_rem = state.stacks.get("bronya_skill_remaining").copied().unwrap_or(0.0);
            if old_t >= 0.0 && old_rem > 0.0 {
                let ot = old_t as usize;
                if ot < state.team.len() {
                    state.team[ot].buffs.dmg_boost -= 66.0;
                }
            }

            // E2 cleanup if a previous E2 SPD boost is still live on the old target
            let e2_t = state.stacks.get("bronya_e2_target").copied().unwrap_or(-1.0);
            if e2_t >= 0.0 {
                let et = e2_t as usize;
                let spd_inc = state.stacks.get("bronya_e2_spd_inc").copied().unwrap_or(0.0);
                if spd_inc > 0.0 && et < state.team.len() {
                    let cur_spd = state.team[et].base_stats
                        .get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
                    state.team[et].base_stats.insert(
                        ids::CHAR_SPD_ID.to_string(), cur_spd - spd_inc,
                    );
                }
                state.stacks.insert("bronya_e2_target".to_string(), -1.0);
                state.stacks.insert("bronya_e2_spd_inc".to_string(), 0.0);
            }

            // Pick the highest-ATK alive non-Bronya ally as skill target
            let target = (0..state.team.len())
                .filter(|&i| i != idx && !state.team[i].is_downed)
                .max_by(|&a, &b| {
                    let atk_a = state.team[a].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
                    let atk_b = state.team[b].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
                    atk_a.partial_cmp(&atk_b).unwrap_or(std::cmp::Ordering::Equal)
                });

            if let Some(t) = target {
                // Apply +66% DMG boost
                state.team[t].buffs.dmg_boost += 66.0;
                state.stacks.insert("bronya_skill_target".to_string(),    t as f64);
                state.stacks.insert("bronya_skill_remaining".to_string(), skill_dur);

                // E2: target ally +30% SPD for 1 turn (applied as flat SPD increase)
                if eidolon >= 2 {
                    let cur_spd = state.team[t].base_stats
                        .get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
                    let inc = cur_spd * 0.30;
                    state.team[t].base_stats.insert(ids::CHAR_SPD_ID.to_string(), cur_spd + inc);
                    state.stacks.insert("bronya_e2_target".to_string(),  t as f64);
                    state.stacks.insert("bronya_e2_spd_inc".to_string(), inc);
                }

                // E1: 50% SP recovery, modelled as +0.5 per skill use via accumulator
                if eidolon >= 1 {
                    let acc = state.stacks.get("bronya_e1_sp_acc").copied().unwrap_or(0.0) + 0.5;
                    if acc >= 1.0 {
                        state.skill_points = (state.skill_points + 1).min(5);
                        state.stacks.insert("bronya_e1_sp_acc".to_string(), acc - 1.0);
                    } else {
                        state.stacks.insert("bronya_e1_sp_acc".to_string(), acc);
                    }
                }

                // Skill: allow target to immediately take action.
                // Drain the AV queue, remove one scheduled entry for this ally,
                // then push an "act now" entry at current_av + ε.
                let target_kit_id = state.team[t].kit_id.clone();
                let current_av    = state.current_av;

                let old_entries: Vec<ActorEntry> = state.av_queue.drain().collect();
                let mut skipped = false;
                for e in old_entries {
                    if !e.is_enemy && e.actor_id == target_kit_id && !skipped {
                        skipped = true; // invalidate the now-stale regular AV entry
                    } else {
                        state.av_queue.push(e);
                    }
                }
                state.av_queue.push(ActorEntry {
                    next_av:     current_av + 0.01,
                    actor_id:    target_kit_id.clone(),
                    instance_id: target_kit_id.clone(),
                    is_enemy:    false,
                });

                let name        = state.team[idx].name.clone();
                let target_name = state.team[t].name.clone();
                state.add_log(&name, format!(
                    "Skill → {} +66% DMG ({:.0}t), Advance Action{}",
                    target_name, skill_dur,
                    if eidolon >= 2 { ", +30% SPD" } else { "" },
                ));
            }
        }
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    // Compute CD bonus from Bronya's current CRIT DMG: 16% × Bronya_CD + 20%
    let bronya_cd = state.team[idx].buffs.crit_dmg; // total CRIT DMG in percent
    let cd_bonus  = bronya_cd * 0.16 + 20.0;

    // Remove existing ult buffs if still active (on refresh)
    let old_cd = state.stacks.get("bronya_ult_cd_bonus").copied().unwrap_or(0.0);
    for i in 0..state.team.len() {
        let rem = state.stacks.get(&ult_rem_key(i)).copied().unwrap_or(0.0);
        if rem > 0.0 {
            remove_ult_buff(state, i, old_cd);
        }
    }
    state.stacks.insert("bronya_ult_cd_bonus".to_string(), cd_bonus);

    // Apply +55% ATK and +cd_bonus CRIT DMG to all alive allies for 2 turns
    for i in 0..state.team.len() {
        if !state.team[i].is_downed {
            apply_ult_buff(state, i, cd_bonus);
        }
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "The Belobog March: all allies +55% ATK, +{:.1}% CRIT DMG (2t)",
        cd_bonus,
    ));
}

#[allow(dead_code)]
pub fn on_break(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,
    source_idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // Tick timed buffs for the ally that just acted
    tick_a4_buff(state, source_idx);
    tick_tech_buff(state, source_idx);
    tick_ult_buff(state, source_idx);

    // Tick skill DMG boost for the skill-buffed ally
    let skill_t = state.stacks.get("bronya_skill_target").copied().unwrap_or(-1.0) as i64;
    if skill_t == source_idx as i64 {
        let rem = state.stacks.get("bronya_skill_remaining").copied().unwrap_or(0.0);
        if rem > 0.0 {
            if rem <= 1.0 {
                if source_idx < state.team.len() {
                    state.team[source_idx].buffs.dmg_boost -= 66.0;
                }
                state.stacks.insert("bronya_skill_target".to_string(),    -1.0);
                state.stacks.insert("bronya_skill_remaining".to_string(),  0.0);
            } else {
                state.stacks.insert("bronya_skill_remaining".to_string(), rem - 1.0);
            }
        }
    }

    // Remove E2 SPD boost after the buffed ally acts
    let e2_t = state.stacks.get("bronya_e2_target").copied().unwrap_or(-1.0) as i64;
    if e2_t == source_idx as i64 {
        let spd_inc = state.stacks.get("bronya_e2_spd_inc").copied().unwrap_or(0.0);
        if spd_inc > 0.0 && source_idx < state.team.len() {
            let cur_spd = state.team[source_idx].base_stats
                .get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
            state.team[source_idx].base_stats.insert(
                ids::CHAR_SPD_ID.to_string(), cur_spd - spd_inc,
            );
        }
        state.stacks.insert("bronya_e2_target".to_string(),  -1.0);
        state.stacks.insert("bronya_e2_spd_inc".to_string(),  0.0);
    }

    // E4: after ally Basic ATK on Wind-weak enemy, Bronya fires a FUA (once per Bronya turn)
    let e4_used = state.stacks.get("bronya_e4_used").copied().unwrap_or(0.0);
    if state.team[idx].eidolon >= 4 && e4_used < 1.0 && action.action_type == ActionType::Basic {
        if let Some(t) = target_idx {
            let is_wind_weak = state.enemies.get(t)
                .and_then(|s| s.as_ref())
                .map_or(false, |e| e.weaknesses.contains(&"Wind".to_string()));
            if is_wind_weak {
                // FUA = 80% of Bronya's Basic ATK DMG (Lv6 Basic = 100% ATK → FUA = 80% ATK)
                let member     = state.team[idx].clone();
                let fua_action = ActionParams {
                    action_type:      ActionType::FollowUp,
                    scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                    multiplier:       0.80,
                    extra_multiplier: 0.0,
                    extra_dmg:        0.0,
                    toughness_damage: 10.0,
                    inflicts_debuff:  false,
                    is_ult_dmg:       false,
                };
                let dmg = state.enemies[t].as_ref()
                    .map(|e| damage::calculate_damage(&member, e, &fua_action))
                    .unwrap_or(0.0);
                if dmg > 0.0 {
                    if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
                    state.total_damage += dmg;
                    if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
                        state.enemies[t] = None;
                    }
                }
                state.stacks.insert("bronya_e4_used".to_string(), 1.0);
                let name = state.team[idx].name.clone();
                state.add_log(&name, format!("E4 FUA: {:.0} DMG (Wind Weakness)", dmg));
            }
        }
    }
}
