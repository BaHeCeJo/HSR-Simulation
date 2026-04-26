use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ─── Stack keys ───────────────────────────────────────────────────────────────
const ENHANCED: &str = "gl_enhanced"; // 1 when next Basic ATK = Nectar Blitz

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn get(state: &SimState, idx: usize, key: &str) -> f64 {
    state.team[idx].stacks.get(key).copied().unwrap_or(0.0)
}

fn set(state: &mut SimState, idx: usize, key: &'static str, v: f64) {
    state.team[idx].stacks.insert(key, v);
}

fn besotted_key(slot: usize) -> String {
    format!("gallagher_besotted_{}", slot)
}

/// Apply or refresh Besotted on an enemy slot. Vulnerability is only added on fresh application.
fn apply_besotted(state: &mut SimState, slot: usize) {
    let key = besotted_key(slot);
    let prev = state.stacks.get(&key).copied().unwrap_or(0.0);
    if prev <= 0.0 {
        if let Some(e) = state.enemies[slot].as_mut() {
            e.vulnerability += 12.0;
        }
    }
    state.stacks.insert(key, 2.0);
}

fn talent_heal_amt(state: &SimState, idx: usize) -> f64 {
    640.0 * (1.0 + state.team[idx].buffs.outgoing_healing / 100.0)
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 110.0;

    // Minor traces: +28% Effect RES, +13.3% BE, +18% HP
    state.team[idx].buffs.effect_res   += 28.0;
    state.team[idx].buffs.break_effect += 13.3;

    let existing_pct = state.team[idx].buffs.hp_percent;
    let hp_bonus = state.team[idx].max_hp * 0.18 / (1.0 + existing_pct / 100.0);
    state.team[idx].max_hp += hp_bonus;
    state.team[idx].hp = state.team[idx].max_hp;
    state.team[idx].buffs.hp_percent += 18.0;

    let eidolon = state.team[idx].eidolon;

    // E1: +20 energy at battle start, +50% Effect RES
    if eidolon >= 1 {
        let max_e = state.team[idx].max_energy;
        state.team[idx].energy = (state.team[idx].energy + 20.0).min(max_e);
        state.team[idx].buffs.effect_res += 50.0;
    }

    // E6: +20% BE, +20% Break Efficiency
    if eidolon >= 6 {
        state.team[idx].buffs.break_effect     += 20.0;
        state.team[idx].buffs.break_efficiency += 20.0;
    }

    // A2: Outgoing Healing = min(total_BE × 0.5, 75%)
    let total_be = state.team[idx].base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0)
        + state.team[idx].buffs.break_effect;
    let oh_bonus = (total_be * 0.5).min(75.0);
    state.team[idx].buffs.outgoing_healing += oh_bonus;

    set(state, idx, ENHANCED, 0.0);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.remove("_action_advance_pct");
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            if get(state, idx, ENHANCED) >= 1.0 {
                // Nectar Blitz: 250% ATK, 30 toughness (Fire)
                action.multiplier       = 2.50;
                action.toughness_damage = 30.0;
            }
            // Normal Basic: simulator defaults (100% ATK, 10 toughness)
        }
        ActionType::Skill => {
            // Pure heal — no damage, no toughness
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
    target_idx: Option<usize>,
) {
    let t_slot = target_idx.or_else(|| {
        state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
    });

    match action.action_type {
        ActionType::Skill => {
            // Heal the ally with the lowest current HP
            let oh = state.team[idx].buffs.outgoing_healing;
            let heal = 1600.0 * (1.0 + oh / 100.0);
            let n = state.team.len();
            let mut target = 0;
            let mut lowest = f64::MAX;
            for i in 0..n {
                if state.team[i].hp < lowest {
                    lowest = state.team[i].hp;
                    target = i;
                }
            }
            let max_hp = state.team[target].max_hp;
            state.team[target].hp = (state.team[target].hp + heal).min(max_hp);
            let tname = state.team[target].name.clone();
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!("Skill heal: +{:.0} HP → {}", heal, tname));
        }

        ActionType::Basic => {
            let enhanced = get(state, idx, ENHANCED) >= 1.0;

            if let Some(t) = t_slot {
                let was_besotted = state.stacks.get(&besotted_key(t)).copied().unwrap_or(0.0) > 0.0;

                if enhanced {
                    // Nectar Blitz: apply/refresh Besotted, then check talent + A6
                    if state.enemies[t].as_ref().map_or(false, |e| e.hp > 0.0) {
                        apply_besotted(state, t);
                        let name = state.team[idx].name.clone();
                        state.add_log(&name, "Nectar Blitz: Besotted applied/refreshed (2t)".to_string());
                    }

                    // Talent heal (Gallagher himself) — only if enemy was already Besotted
                    if was_besotted {
                        let heal = talent_heal_amt(state, idx);
                        let max_hp = state.team[idx].max_hp;
                        state.team[idx].hp = (state.team[idx].hp + heal).min(max_hp);

                        // A6: heal all other allies when Enhanced Basic hits Besotted
                        let oh = state.team[idx].buffs.outgoing_healing;
                        let a6_heal = 640.0 * (1.0 + oh / 100.0);
                        let n = state.team.len();
                        for i in 0..n {
                            if i == idx { continue; }
                            let max_hp_i = state.team[i].max_hp;
                            state.team[i].hp = (state.team[i].hp + a6_heal).min(max_hp_i);
                        }
                        let name = state.team[idx].name.clone();
                        state.add_log(&name, format!(
                            "Talent +A6: Gallagher healed +{:.0} HP; allies healed +{:.0} HP each",
                            heal, a6_heal,
                        ));
                    }

                    set(state, idx, ENHANCED, 0.0);
                } else {
                    // Normal Basic hitting a Besotted enemy → talent heal
                    if was_besotted {
                        let heal = talent_heal_amt(state, idx);
                        let max_hp = state.team[idx].max_hp;
                        state.team[idx].hp = (state.team[idx].hp + heal).min(max_hp);
                        let name = state.team[idx].name.clone();
                        state.add_log(&name, format!("Talent: self healed +{:.0} HP (Besotted)", heal));
                    }
                }
            } else if enhanced {
                set(state, idx, ENHANCED, 0.0);
            }
        }

        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);

    let name = state.team[idx].name.clone();
    let mut total_dmg = 0.0;
    let enemy_count = state.enemies.iter()
        .filter(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
        .count();

    for slot in 0..state.enemies.len() {
        if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }

        let ult_action = ActionParams {
            action_type:      ActionType::Ultimate,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       1.50,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 20.0,
            inflicts_debuff:  false,
            is_ult_dmg:       true,
        };

        let dmg = {
            let m = &state.team[idx];
            state.enemies[slot].as_ref()
                .map(|e| damage::calculate_damage(m, e, &ult_action))
                .unwrap_or(0.0)
        };

        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            state.total_damage += dmg;
            total_dmg += dmg;
            if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[slot] = None;
            }
        }

        // Apply Besotted to surviving enemies (ult hit itself doesn't trigger talent heal)
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp > 0.0) {
            apply_besotted(state, slot);
        }
    }

    state.add_log(&name, format!(
        "Ult: {:.0} DMG ({} targets) | Besotted applied to all",
        total_dmg, enemy_count,
    ));

    // Restore 5 energy (energy was zeroed before on_ult)
    state.team[idx].energy = 5.0;

    // Grant Enhanced Basic
    set(state, idx, ENHANCED, 1.0);

    // A4: 100% action advance → 99.0 (engine clips at < 100)
    set(state, idx, "_action_advance_pct", 99.0);
}

pub fn on_enemy_turn_start(state: &mut SimState, idx: usize, enemy_idx: usize) {
    // Tick Besotted duration; revert vulnerability on expiry
    let key = besotted_key(enemy_idx);
    let turns = state.stacks.get(&key).copied().unwrap_or(0.0);
    if turns <= 0.0 { return; }

    let new_turns = turns - 1.0;
    if new_turns <= 0.0 {
        state.stacks.remove(&key);
        if let Some(e) = state.enemies[enemy_idx].as_mut() {
            e.vulnerability -= 12.0;
        }
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Besotted expired (enemy {})", enemy_idx));
    } else {
        state.stacks.insert(key, new_turns);
    }
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,
    source_idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // Talent: when an ally attacks a Besotted enemy, that ally heals
    let is_attack = matches!(
        action.action_type,
        ActionType::Basic | ActionType::Skill | ActionType::Ultimate | ActionType::FollowUp
    );
    if !is_attack { return; }

    let t = match target_idx {
        Some(t) => t,
        None    => return,
    };

    if state.stacks.get(&besotted_key(t)).copied().unwrap_or(0.0) <= 0.0 { return; }

    let oh = state.team[idx].buffs.outgoing_healing;
    let heal = 640.0 * (1.0 + oh / 100.0);
    let max_hp = state.team[source_idx].max_hp;
    state.team[source_idx].hp = (state.team[source_idx].hp + heal).min(max_hp);
    let name = state.team[idx].name.clone();
    let aname = state.team[source_idx].name.clone();
    state.add_log(&name, format!("Talent: {} healed +{:.0} HP (Besotted)", aname, heal));
}
