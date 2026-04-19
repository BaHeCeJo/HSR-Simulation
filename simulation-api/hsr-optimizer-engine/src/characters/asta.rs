use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

const CHARGING_KEY: &str = "asta_charging";
const E2_SKIP_KEY: &str  = "asta_e2_skip";
const TURN_KEY: &str     = "asta_turn";

fn set_charging(state: &mut SimState, new_stacks: f64) {
    let old = state.stacks.get(CHARGING_KEY).copied().unwrap_or(0.0);
    let capped = new_stacks.clamp(0.0, 5.0);
    let diff = (capped - old) * 14.0;
    if diff.abs() > 0.001 {
        for m in state.team.iter_mut() {
            if !m.is_downed { m.buffs.atk_percent += diff; }
        }
    }
    state.stacks.insert(CHARGING_KEY.to_string(), capped);
}

fn decrement_spd_buff(state: &mut SimState, member_idx: usize) {
    let key = format!("asta_spd_remaining_{}", state.team[member_idx].kit_id);
    let remaining = state.stacks.get(&key).copied().unwrap_or(0.0);
    if remaining <= 0.0 { return; }
    if remaining <= 1.0 {
        // Revert SPD
        let spd = state.team[member_idx].base_stats
            .entry(ids::CHAR_SPD_ID.to_string())
            .or_insert(100.0);
        *spd -= 50.0;
        state.stacks.remove(&key);
    } else {
        state.stacks.insert(key, remaining - 1.0);
    }
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy        = 120.0;
    state.team[idx].buffs.dmg_boost  += 22.4; // minor trace: Fire DMG +22.4%
    state.team[idx].buffs.def_percent += 22.5; // minor trace: DEF +22.5%
    state.team[idx].buffs.crit_rate  +=  6.7; // minor trace: CRIT Rate +6.7%

    state.stacks.insert(CHARGING_KEY.to_string(), 0.0);
    state.stacks.insert(TURN_KEY.to_string(), 0.0);

    // A4: All Fire allies +18% Fire DMG
    for m in state.team.iter_mut() {
        if m.element == "Fire" {
            m.buffs.dmg_boost += 18.0;
        }
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    let turn = state.stacks.get(TURN_KEY).copied().unwrap_or(0.0) + 1.0;
    state.stacks.insert(TURN_KEY.to_string(), turn);

    if turn >= 2.0 {
        if state.stacks.get(E2_SKIP_KEY).copied().unwrap_or(0.0) >= 1.0 {
            state.stacks.insert(E2_SKIP_KEY.to_string(), 0.0);
        } else {
            let eidolon = state.team[idx].eidolon;
            let reduction = if eidolon >= 6 { 2.0 } else { 3.0 };
            let current = state.stacks.get(CHARGING_KEY).copied().unwrap_or(0.0);
            if current > 0.0 {
                set_charging(state, (current - reduction).max(0.0));
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
    // Correct energy: skill should only gain +6 net (simulator will add +30)
    if action.action_type == ActionType::Skill {
        state.team[idx].energy = (state.team[idx].energy - 24.0).max(0.0);
    }

    // A6: +6% DEF per Charging stack
    let charging = state.stacks.get(CHARGING_KEY).copied().unwrap_or(0.0);
    state.team[idx].buffs.def_percent += charging * 6.0;

    // E4: +15% ERR when Charging >= 2
    if state.team[idx].eidolon >= 4 && charging >= 2.0 {
        state.team[idx].buffs.energy_regen_rate += 15.0;
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // A2 Burn: 50% of Basic ATK DMG per tick
    let eff_atk = state.team[idx].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0)
        * (1.0 + state.team[idx].buffs.atk_percent / 100.0);
    let burn_val = eff_atk * 0.5;
    let ehr = state.team[idx].base_stats.get(ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
            + state.team[idx].buffs.effect_hit_rate;

    match action.action_type {
        ActionType::Basic => {
            if let Some(t) = target_idx {
                if let Some(enemy) = state.enemies[t].as_ref() {
                    let fire_weak = enemy.weaknesses.contains(&"Fire".to_string());
                    let gain = if fire_weak { 2.0 } else { 1.0 };
                    let current = state.stacks.get(CHARGING_KEY).copied().unwrap_or(0.0);
                    set_charging(state, current + gain);
                }
                // A2: Burn on Basic hit (80% base chance)
                if let Some(enemy) = state.enemies[t].as_mut() {
                    effects::try_apply_enemy_debuff(ehr, enemy, "asta_burn", StatusEffect {
                        duration: 3,
                        value:    burn_val,
                        stat:     Some("Burn".to_string()),
                        effects:  vec![],
                    }, 0.8);
                }
            }
            decrement_spd_buff(state, idx);
        }
        ActionType::Skill => {
            // Fire 4 extra bounce hits (5 at E1)
            let eidolon = state.team[idx].eidolon;
            let extra_bounces = if eidolon >= 1 { 5 } else { 4 };
            let member = state.team[idx].clone();
            let bounce_action = ActionParams {
                action_type:      ActionType::Skill,
                scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                multiplier:       0.5,
                extra_multiplier: 0.0,
                extra_dmg:        0.0,
                toughness_damage: 0.0,
                inflicts_debuff:  false,
                is_ult_dmg:       false,
            };

            let mut hit_ids: Vec<String> = Vec::new();
            if let Some(t) = target_idx {
                if let Some(e) = state.enemies[t].as_ref() {
                    hit_ids.push(e.instance_id.clone());
                }
            }

            for k in 0..extra_bounces {
                // Find alive enemies
                let alive: Vec<usize> = state.enemies.iter().enumerate()
                    .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
                    .collect();
                if alive.is_empty() { break; }
                // Prefer not-yet-hit enemies
                let pick = alive.iter()
                    .find(|&&i| !hit_ids.contains(
                        &state.enemies[i].as_ref().map(|e| e.instance_id.clone()).unwrap_or_default()
                    ))
                    .copied()
                    .unwrap_or(alive[k % alive.len()]);

                let bounce_dmg = state.enemies[pick].as_ref()
                    .map(|e| {
                        hit_ids.push(e.instance_id.clone());
                        damage::calculate_damage(&member, e, &bounce_action)
                    })
                    .unwrap_or(0.0);
                if bounce_dmg > 0.0 {
                    if let Some(e) = state.enemies[pick].as_mut() { e.hp -= bounce_dmg; }
                    state.total_damage += bounce_dmg;
                }
                if state.enemies[pick].as_ref().map_or(false, |e| e.hp <= 0.0) {
                    state.enemies[pick] = None;
                }
            }

            // Charging stacks: 1 per unique enemy hit + 1 if Fire Weakness
            let fire_key = "Fire".to_string();
            let mut gain = 0.0f64;
            for iid in &hit_ids {
                gain += 1.0;
                // Check fire weakness for this enemy instance
                let is_fire_weak = state.enemies.iter()
                    .filter_map(|s| s.as_ref())
                    .find(|e| &e.instance_id == iid)
                    .map_or(false, |e| e.weaknesses.contains(&fire_key));
                if is_fire_weak { gain += 1.0; }
            }
            let current = state.stacks.get(CHARGING_KEY).copied().unwrap_or(0.0);
            set_charging(state, current + gain);

            decrement_spd_buff(state, idx);
        }
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].energy = 5.0;

    // E2: skip next charging reduction
    if state.team[idx].eidolon >= 2 {
        state.stacks.insert(E2_SKIP_KEY.to_string(), 1.0);
    }

    // Apply SPD +50 to all alive allies for 2 turns (2 of their own actions)
    let kit_ids: Vec<String> = state.team.iter()
        .filter(|m| !m.is_downed)
        .map(|m| m.kit_id.clone())
        .collect();

    for kit_id in &kit_ids {
        // Find member index
        if let Some(midx) = state.team.iter().position(|m| &m.kit_id == kit_id) {
            *state.team[midx].base_stats.entry(ids::CHAR_SPD_ID.to_string()).or_insert(100.0) += 50.0;
            let key = format!("asta_spd_remaining_{}", kit_id);
            state.stacks.insert(key, 2.0);
        }
    }
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(state: &mut SimState, _idx: usize, enemy_idx: usize) {
    // Apply Burn DoT
    let burn_val = state.enemies[enemy_idx].as_ref()
        .and_then(|e| e.active_debuffs.get("asta_burn"))
        .map(|b| b.value)
        .unwrap_or(0.0);
    if burn_val <= 0.0 { return; }

    if let Some(e) = state.enemies[enemy_idx].as_mut() {
        e.hp -= burn_val;
        if e.hp <= 0.0 { state.enemies[enemy_idx] = None; }
    }
    state.total_damage += burn_val;
}

pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state: &mut SimState,
    _idx: usize,
    source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {
    decrement_spd_buff(state, source_idx);
}
