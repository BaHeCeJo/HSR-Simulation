use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

/// Element order used to encode the implanted weakness as an f64 index.
const SW_ELEMENTS: &[&str] = &[
    "Physical","Fire","Ice","Lightning","Wind","Quantum","Imaginary",
];

// ── Bug helpers ───────────────────────────────────────────────────────────────

fn bug_key_and_stat(debuff_count: u32) -> (&'static str, &'static str) {
    match debuff_count % 3 {
        0 => ("sw_bug_atk", "Bug"),        // ATK -10%  (no dmg formula impact)
        1 => ("sw_bug_def", "DEF Reduction"), // DEF -12%  (picked up by def_mult)
        _ => ("sw_bug_spd", "Bug"),        // SPD -6%   (no dmg formula impact)
    }
}

fn apply_random_bug(ehr: f64, enemy: &mut crate::models::SimEnemy, base_chance: f64) {
    let (key, stat) = bug_key_and_stat(enemy.debuff_count);
    effects::try_apply_enemy_debuff(ehr, enemy, key, StatusEffect {
        duration: 4, // A2: base 3 + 1 extension
        value: 10.0,
        stat: Some(stat.to_string()),
        effects: vec![],
    }, base_chance);
}

// ── Weakness implant (Skill) ──────────────────────────────────────────────────

/// Add the first alive ally's element as a weakness on the enemy (120% base
/// chance). Tracks which element SW implanted per enemy so it can be removed
/// if she implants again.
fn sw_implant_weakness(state: &mut SimState, idx: usize, enemy_slot: usize) {
    let ehr = state.team[idx].base_stats.get(ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
            + state.team[idx].buffs.effect_hit_rate;

    let elem = match state.team.iter().find(|m| !m.is_downed).map(|m| m.element.clone()) {
        Some(e) => e,
        None    => return,
    };
    let (iid, enemy_res) = match state.enemies[enemy_slot].as_ref() {
        Some(e) => (e.instance_id.clone(), e.effect_res),
        None    => return,
    };
    if !effects::debuff_lands(ehr, enemy_res, 1.2) { return; }

    let wk_key = format!("sw_wk_elem_{}", iid);

    // Remove previously implanted weakness if any
    if let Some(old_idx) = state.stacks.get(&wk_key).copied() {
        if let Some(&old_elem) = SW_ELEMENTS.get(old_idx as usize) {
            if let Some(e) = state.enemies[enemy_slot].as_mut() {
                e.weaknesses.retain(|w| w.as_str() != old_elem);
            }
        }
    }

    let new_idx = SW_ELEMENTS.iter().position(|&e| e == elem.as_str()).unwrap_or(99) as f64;
    state.stacks.insert(wk_key.to_string(), new_idx);

    if let Some(e) = state.enemies[enemy_slot].as_mut() {
        if !e.weaknesses.contains(&elem) {
            e.weaknesses.push(elem);
        }
    }
}

// ── Hooks ─────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy            = 110.0;
    state.team[idx].buffs.atk_percent     += 28.0;  // minor trace: ATK +28%
    state.team[idx].buffs.dmg_boost       +=  8.0;  // minor trace: Quantum DMG +8%
    state.team[idx].buffs.effect_hit_rate += 18.0;  // minor trace: Effect HIT Rate +18%
    // A4: start with 20 energy; +5 per turn handled in on_turn_start
    state.team[idx].energy = 20.0;

    // E2: all enemies enter battle with +20% vulnerability
    if state.team[idx].eidolon >= 2 {
        for slot in state.enemies.iter_mut() {
            if let Some(e) = slot.as_mut() {
                e.vulnerability += 20.0;
            }
        }
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // A4: regenerate 5 energy at the start of each turn
    state.team[idx].energy = (state.team[idx].energy + 5.0).min(state.team[idx].max_energy);
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    // A6: per 10% EHR → +10% ATK (max +50%)
    let ehr = state.team[idx].base_stats.get(ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
            + state.team[idx].buffs.effect_hit_rate;
    let atk_gain = ((ehr / 10.0).floor() * 10.0).min(50.0);
    state.team[idx].buffs.atk_percent += atk_gain;

    // E6: +20% DMG per debuff on target (max +100%)
    if state.team[idx].eidolon >= 6 {
        if let Some(t) = target_idx {
            if let Some(enemy) = state.enemies[t].as_ref() {
                state.team[idx].buffs.dmg_boost += (enemy.debuff_count as f64 * 20.0).min(100.0);
            }
        }
    }

    if action.action_type == ActionType::Skill {
        action.inflicts_debuff = true;
        if let Some(t) = target_idx {
            // Implant weakness (120% base chance, first alive ally's element)
            sw_implant_weakness(state, idx, t);
            if let Some(enemy) = state.enemies[t].as_mut() {
                // -20% RES to that weakness type (picked up by "Weakness RES" in damage formula)
                effects::try_apply_enemy_debuff(ehr, enemy, "sw_weakness_res", StatusEffect {
                    duration: 3, value: 20.0,
                    stat: Some("Weakness RES".to_string()), effects: vec![],
                }, 1.0);
                // -13% All-Type RES
                effects::try_apply_enemy_debuff(ehr, enemy, "sw_all_res", StatusEffect {
                    duration: 2, value: 13.0,
                    stat: Some("All RES".to_string()), effects: vec![],
                }, 1.0);
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
    if !matches!(action.action_type, ActionType::Basic | ActionType::Skill | ActionType::Ultimate) {
        return;
    }

    let ehr = state.team[idx].base_stats.get(ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
            + state.team[idx].buffs.effect_hit_rate;
    let eidolon = state.team[idx].eidolon;

    // For ult AoE, the original target may be dead; fall back to first alive enemy.
    let bug_target = target_idx
        .filter(|&t| state.enemies[t].as_ref().map_or(false, |e| e.hp > 0.0))
        .or_else(|| state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0)));

    if let Some(t) = bug_target {
        if let Some(enemy) = state.enemies[t].as_mut() {
            // Talent: 1 random Bug after each SW action
            apply_random_bug(ehr, enemy, 1.0);
            // E2: SW's own attacks also trigger E2 (dispatch skips self for on_ally_action)
            if eidolon >= 2 {
                apply_random_bug(ehr, enemy, 1.0);
            }
        }
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;
    let ehr = state.team[idx].base_stats.get(ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
            + state.team[idx].buffs.effect_hit_rate;

    // Apply -45% DEF to all enemies (120% base chance → guaranteed)
    for slot in state.enemies.iter_mut() {
        if let Some(enemy) = slot.as_mut() {
            effects::try_apply_enemy_debuff(ehr, enemy, "sw_ult_def", StatusEffect {
                duration: 3, value: 45.0,
                stat: Some("DEF Reduction".to_string()), effects: vec![],
            }, 1.2);
        }
    }

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if alive.is_empty() { return; }

    let member = state.team[idx].clone();
    let ult_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       3.80,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 30.0,
        inflicts_debuff:  true,
        is_ult_dmg:       true,
    };

    let mut total = 0.0f64;
    let mut max_debuffs = 0u32;

    for &i in &alive {
        // Capture debuff count before damage (for E1 energy, include the DEF debuff just applied)
        let dc = state.enemies[i].as_ref().map(|e| e.debuff_count).unwrap_or(0);
        if dc > max_debuffs { max_debuffs = dc; }

        // Main hit: 380% ATK AoE
        let dmg = state.enemies[i].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &ult_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[i].as_mut() { e.hp -= dmg; }
            total += dmg;
        }

        // E4: additional DMG per debuff (20% ATK per hit, max 5 hits per enemy)
        if eidolon >= 4 {
            let hits = (dc as usize).min(5);
            let e4_action = ActionParams { multiplier: 0.20, toughness_damage: 0.0, ..ult_action.clone() };
            for _ in 0..hits {
                if state.enemies[i].as_ref().map_or(true, |e| e.hp <= 0.0) { break; }
                let e4_dmg = state.enemies[i].as_ref()
                    .map(|e| damage::calculate_damage(&member, e, &e4_action))
                    .unwrap_or(0.0);
                if e4_dmg > 0.0 {
                    if let Some(e) = state.enemies[i].as_mut() { e.hp -= e4_dmg; }
                    total += e4_dmg;
                }
            }
        }

        if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[i] = None;
        }
    }
    state.total_damage += total;

    // E1: +7 energy per debuff on primary target (max 5 triggers = 35 energy)
    if eidolon >= 1 {
        let triggers = (max_debuffs as usize).min(5);
        state.team[idx].energy =
            (state.team[idx].energy + triggers as f64 * 7.0).min(state.team[idx].max_energy);
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("User Banned (AoE): {:.0} DMG", total));
}

/// A2: 100% base chance to implant a random Bug when an enemy is Weakness Broken.
pub fn on_break(state: &mut SimState, idx: usize, enemy_idx: usize) {
    let ehr = state.team[idx].base_stats.get(ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
            + state.team[idx].buffs.effect_hit_rate;
    if let Some(enemy) = state.enemies[enemy_idx].as_mut() {
        apply_random_bug(ehr, enemy, 1.0);
    }
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
    state: &mut SimState,
    idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    target_idx: Option<usize>,
) {
    // E2: 100% base chance to implant random Bug when any ally attacks an enemy.
    // (SW's own attacks are covered separately in on_after_action.)
    if state.team[idx].eidolon >= 2 {
        let ehr = state.team[idx].base_stats.get(ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
                + state.team[idx].buffs.effect_hit_rate;
        if let Some(t) = target_idx {
            if let Some(enemy) = state.enemies[t].as_mut() {
                apply_random_bug(ehr, enemy, 1.0);
            }
        }
    }
}
