use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

const ENERGY_KEY: &str    = "anaxa_energy";
const ENERGY_CAP: f64     = 140.0;
const ALL_ELEMENTS: &[&str] = &["Physical","Fire","Ice","Lightning","Wind","Quantum","Imaginary"];

// ── Energy helpers ────────────────────────────────────────────────────────────

fn add_energy(state: &mut SimState, idx: usize, amount: f64) {
    let cur = state.stacks.get(ENERGY_KEY).copied().unwrap_or(0.0);
    state.stacks.insert(ENERGY_KEY.to_string(), (cur + amount).min(ENERGY_CAP));
    if state.stacks.get(ENERGY_KEY).copied().unwrap_or(0.0) >= ENERGY_CAP {
        state.team[idx].stacks.insert("_ult_ready", 1.0);
    }
}

// ── Weakness / QD helpers ─────────────────────────────────────────────────────

fn wk_key(instance_id: &str) -> String {
    format!("anaxa_wk_{}", instance_id)
}

fn get_wk_count(state: &SimState, instance_id: &str) -> usize {
    state.stacks.get(&wk_key(instance_id)).copied().unwrap_or(0.0) as usize
}

fn add_wk_count(state: &mut SimState, instance_id: &str, n: usize) -> usize {
    let prev = get_wk_count(state, instance_id);
    let next = (prev + n).min(7);
    state.stacks.insert(wk_key(instance_id), next as f64);
    next
}

fn has_qd(state: &SimState, instance_id: &str) -> bool {
    get_wk_count(state, instance_id) >= 5
}

/// Implant n weakness types on an enemy, updating its weaknesses list and QD status.
fn implant_weakness(state: &mut SimState, enemy_slot: usize, n: usize) {
    let iid = match state.enemies[enemy_slot].as_ref() {
        Some(e) => e.instance_id.clone(),
        None    => return,
    };
    let prev = get_wk_count(state, &iid);
    let next = add_wk_count(state, &iid, n);

    // Append concrete weakness strings
    for i in prev..next {
        if let Some(&elem) = ALL_ELEMENTS.get(i) {
            if let Some(e) = state.enemies[enemy_slot].as_mut() {
                if !e.weaknesses.contains(&elem.to_string()) {
                    e.weaknesses.push(elem.to_string());
                }
            }
        }
    }

    // First time ≥5: mark Qualitative Disclosure
    if next >= 5 {
        let qd_key = format!("anaxa_qd_active_{}", iid);
        if state.stacks.get(&qd_key).copied().unwrap_or(0.0) < 1.0 {
            state.stacks.insert(qd_key.to_string(), 1.0);
        }
    }
}

/// Execute skill bounce hits (hitcount hits at `skill_mult`, implant weakness per hit).
/// Returns the enemy slot of the main target (first alive enemy).
fn execute_skill_bounces(
    state: &mut SimState, idx: usize, skill_mult: f64, hit_count: usize,
) {
    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if alive.is_empty() { return; }

    let alive_count = alive.len();
    let per_enemy_bonus = alive_count as f64 * 20.0; // Skill: +20% DMG per attackable enemy
    let member = state.team[idx].clone();

    let mut hit_slots: Vec<usize> = Vec::new();
    if let Some(&first) = alive.first() {
        hit_slots.push(first);
    }
    for _ in 1..hit_count {
        let live: Vec<usize> = state.enemies.iter().enumerate()
            .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
            .collect();
        if live.is_empty() { break; }
        let unhit: Vec<usize> = live.iter().copied().filter(|i| !hit_slots.contains(i)).collect();
        let pick = if !unhit.is_empty() { unhit[0] } else { live[hit_slots.len() % live.len()] };
        hit_slots.push(pick);
    }

    let action = ActionParams {
        action_type:      ActionType::Skill,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       skill_mult,
        extra_multiplier: per_enemy_bonus, // additive DMG boost built into extra_mult field
        extra_dmg:        0.0,
        toughness_damage: 10.0,
        inflicts_debuff:  true,
        is_ult_dmg:       false,
    };

    for &slot in &hit_slots {
        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            state.total_damage += dmg;
        }
        implant_weakness(state, slot, 1);
        // E1: DEF shred on every Skill hit (-16% DEF for 2 turns)
        if state.team[idx].eidolon >= 1 {
            if let Some(e) = state.enemies[slot].as_mut() {
                effects::apply_enemy_debuff(e, "anaxa_e1_def", StatusEffect {
                    duration: 2, value: 16.0,
                    stat: Some("DEF Reduction".to_string()), effects: vec![],
                });
            }
        }
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[slot] = None;
        }
    }
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy     = f64::MAX; // custom energy
    state.team[idx].buffs.crit_rate  += 12.0;  // minor trace: CRIT Rate +12%
    state.team[idx].buffs.hp_percent += 10.0;  // minor trace: HP +10%
    state.team[idx].buffs.dmg_boost  += 22.4;  // minor trace: Wind DMG +22.4%

    state.stacks.insert(ENERGY_KEY.to_string(), 0.0);

    // A4: Imperative Hiatus
    // 1+ Erudition (Anaxa herself counts): +140% CRIT DMG
    state.team[idx].buffs.crit_dmg += 140.0;

    // 2+ Erudition: +50% DMG to all allies (simulated as enemy vulnerability +50%)
    let erudition_count = state.team.iter()
        .filter(|m| m.path == "Erudition").count();
    let two_effect = state.team[idx].eidolon >= 6 || erudition_count >= 2;
    if two_effect {
        for slot in state.enemies.iter_mut() {
            if let Some(e) = slot.as_mut() { e.vulnerability += 50.0; }
        }
    }

    // E6: +30% DMG boost (approximation of ×1.3 multiplicative)
    if state.team[idx].eidolon >= 6 {
        state.team[idx].buffs.dmg_boost += 30.0;
    }

    // E2: weakness implant + -20% All-Type RES on every enemy
    if state.team[idx].eidolon >= 2 {
        for i in 0..state.enemies.len() {
            implant_weakness(state, i, 1);
            if let Some(e) = state.enemies[i].as_mut() {
                e.resistance = (e.resistance - 0.20).max(-1.0);
                for res in e.elemental_res.values_mut() {
                    *res = (*res - 0.20).max(-1.0);
                }
            }
        }
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // A2: if no QD enemy → +30 energy
    let any_qd = state.enemies.iter()
        .filter_map(|s| s.as_ref())
        .any(|e| has_qd(state, &e.instance_id));
    if !any_qd {
        add_energy(state, idx, 30.0);
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    if let Some(t) = target_idx {
        if let Some(enemy) = state.enemies[t].as_ref() {
            let iid = enemy.instance_id.clone();

            // QD: +30% DMG
            if has_qd(state, &iid) {
                state.team[idx].buffs.dmg_boost += 30.0;
            }

            // A6: +4% DEF ignore per Weakness Type (max 7)
            let wk = get_wk_count(state, &iid).min(7);
            state.team[idx].buffs.def_ignore += wk as f64 * 4.0;

            // E1: DEF shred is already on enemy.active_debuffs ("anaxa_e1_def",
            // stat: "DEF Reduction") so damage::calculate_damage picks it up via
            // enemy_def_reduce. No attacker-side buff needed — adding it here would
            // double-count the 16%.
        }
    }

    // Skill: +20% DMG per alive enemy
    if action.action_type == ActionType::Skill {
        let alive = state.enemies.iter().filter(|s| s.as_ref().map_or(false, |e| e.hp > 0.0)).count();
        action.extra_multiplier += alive as f64 * 20.0;
    }

    action.inflicts_debuff = true;
    // Prevent simulator from adding energy (managed manually)
    state.team[idx].energy = 0.0;
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            if let Some(t) = target_idx {
                implant_weakness(state, t, 1);

                // QD extra Skill triggered inline (no extra-turn system in Rust)
                let iid = state.enemies[t].as_ref()
                    .map(|e| e.instance_id.clone())
                    .unwrap_or_default();
                let trigger_key = format!("anaxa_qd_trigger_{}", state.current_action_id);
                if has_qd(state, &iid) && state.stacks.get(&trigger_key).copied().unwrap_or(0.0) < 1.0 {
                    state.stacks.insert(trigger_key.to_string(), 1.0);
                    // Execute 5 QD extra skill hits at 70%
                    execute_skill_bounces(state, idx, 0.70, 5);
                    add_energy(state, idx, 6.0);
                }
            }
            add_energy(state, idx, 30.0); // 20 base + 10 A2
            state.team[idx].energy = 0.0;
        }
        ActionType::Skill => {
            if let Some(t) = target_idx {
                // Resolve 4 bounce hits (hit 0 was done by simulator, implant its weakness)
                implant_weakness(state, t, 1);
                if state.team[idx].eidolon >= 1 {
                    if let Some(e) = state.enemies[t].as_mut() {
                        effects::apply_enemy_debuff(e, "anaxa_e1_def", StatusEffect {
                            duration: 2, value: 16.0,
                            stat: Some("DEF Reduction".to_string()), effects: vec![],
                        });
                    }
                }
                execute_skill_bounces(state, idx, 0.70, 4); // 4 bounce hits

                // E1: SP refund on first Skill of the battle
                if state.team[idx].eidolon >= 1
                    && state.stacks.get("anaxa_e1_sp_done").copied().unwrap_or(0.0) < 1.0
                {
                    state.stacks.insert("anaxa_e1_sp_done".to_string(), 1.0);
                    state.skill_points = (state.skill_points + 1).min(5);
                }

                // QD extra Skill triggered inline
                let iid = state.enemies[t].as_ref()
                    .map(|e| e.instance_id.clone())
                    .unwrap_or_default();
                let trigger_key = format!("anaxa_qd_trigger_{}", state.current_action_id);
                if has_qd(state, &iid) && state.stacks.get(&trigger_key).copied().unwrap_or(0.0) < 1.0 {
                    state.stacks.insert(trigger_key.to_string(), 1.0);
                    execute_skill_bounces(state, idx, 0.70, 5);
                    add_energy(state, idx, 6.0);
                }
            }
            add_energy(state, idx, 6.0);
            state.team[idx].energy = 0.0;
        }
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].stacks.remove("_ult_ready");
    state.stacks.insert(ENERGY_KEY.to_string(), 5.0);
    state.team[idx].energy = 0.0;

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if alive.is_empty() { return; }

    // Sublimation: implant all 7 weakness types on all alive enemies
    for &slot in &alive {
        let prev = state.enemies[slot].as_ref()
            .map(|e| get_wk_count(state, &e.instance_id)).unwrap_or(0);
        if prev < 7 {
            implant_weakness(state, slot, 7 - prev);
        }
    }

    // AoE 160% ATK DMG to all alive enemies
    let member = state.team[idx].clone();
    let ult_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       1.60,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  true,
        is_ult_dmg:       true,
    };

    let mut total = 0.0f64;
    for &slot in &alive {
        // Apply QD bonus inline: all enemies now have QD after Sublimation
        let mut hit_member = member.clone();
        hit_member.buffs.dmg_boost += 30.0; // QD: +30% DMG (all enemies now have ≥5 types)

        let iid = state.enemies[slot].as_ref().map(|e| e.instance_id.clone()).unwrap_or_default();
        let wk = get_wk_count(state, &iid).min(7);
        hit_member.buffs.def_ignore += wk as f64 * 4.0; // A6

        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&hit_member, e, &ult_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            total += dmg;
        }
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[slot] = None;
        }
    }
    state.total_damage += total;

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Sprouting Life Sculpts Earth: {:.0} DMG", total));
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
