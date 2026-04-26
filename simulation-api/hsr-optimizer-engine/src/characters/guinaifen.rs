use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimEnemy, SimState, TeamMember};

// ─── Stack keys ───────────────────────────────────────────────────────────────
const BURN_ACC:    &str = "gn_burn_acc"; // A2: accumulates 0.80 per Basic → Burn at ≥ 1.0
const A6_APPLIED:  &str = "gn_a6";      // 1 if +20% dmg_boost was added for the current action

// ─── Per-enemy state (state.stacks, String keys) ─────────────────────────────
fn burn_key(s: usize)       -> String { format!("guinaifen_burn_{}", s) }       // turns remaining
fn burn_boost_key(s: usize) -> String { format!("guinaifen_burn_boost_{}", s) } // E2: +40pp
fn fk_key(s: usize)         -> String { format!("guinaifen_fk_{}", s) }         // Firekiss stacks
fn fk_dur_key(s: usize)     -> String { format!("guinaifen_fk_dur_{}", s) }     // Firekiss turns

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn get(state: &SimState, idx: usize, key: &str) -> f64 {
    state.team[idx].stacks.get(key).copied().unwrap_or(0.0)
}

fn set(state: &mut SimState, idx: usize, key: &'static str, v: f64) {
    state.team[idx].stacks.insert(key, v);
}

fn is_burned(state: &SimState, slot: usize) -> bool {
    state.stacks.get(&burn_key(slot)).copied().unwrap_or(0.0) > 0.0
}

/// Burn DoT DMG: ATK-scaling, **no crit** (cr zeroed), A6 +20% always included.
/// `mult` = 2.182 + E2 boost / 100 — already computed by caller.
fn calc_burn_dmg(member: &TeamMember, target: &SimEnemy, mult: f64) -> f64 {
    let mut dot = member.clone();
    dot.buffs.crit_rate  = 0.0;  // DoTs never crit → expected_crit = 1.0
    dot.buffs.dmg_boost += 20.0; // A6: target is Burned by definition when DoT fires
    let action = ActionParams {
        action_type:      ActionType::TalentProc,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       mult,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 0.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };
    damage::calculate_damage(&dot, target, &action)
}

/// Deal one Burn DMG instance, apply Talent (Firekiss) if enemy survives,
/// and grant Talent (+5) + E4 (+2) energy to Guinaifen. Returns DMG dealt.
fn burn_proc(state: &mut SimState, idx: usize, slot: usize, proc_mult: f64) -> f64 {
    if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { return 0.0; }
    let eidolon = state.team[idx].eidolon;

    let dmg = {
        let m = &state.team[idx];
        state.enemies[slot].as_ref()
            .map(|e| calc_burn_dmg(m, e, proc_mult))
            .unwrap_or(0.0)
    };
    if dmg > 0.0 {
        if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[slot] = None;
        }
    }

    // Talent: Firekiss on every Burn DMG instance (alive targets only)
    if state.enemies[slot].as_ref().map_or(false, |e| e.hp > 0.0) {
        let max_fk = if eidolon >= 6 { 4.0 } else { 3.0 };
        apply_firekiss(state, slot, max_fk);
    }

    // Talent +5 energy, E4 +2 energy per Burn proc
    let err = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
    let energy_gain = (5.0 + if eidolon >= 4 { 2.0 } else { 0.0 }) * err;
    let max_e = state.team[idx].max_energy;
    state.team[idx].energy = (state.team[idx].energy + energy_gain).min(max_e);

    dmg
}

/// Apply or refresh Burn (2 turns). Does NOT reset the E2 boost (boost persists on refresh).
fn apply_burn(state: &mut SimState, slot: usize) {
    state.stacks.insert(burn_key(slot), 2.0);
}

/// Add one Firekiss stack (max 3, or 4 with E6) and refresh duration to 3.
fn apply_firekiss(state: &mut SimState, slot: usize, max_stacks: f64) {
    let cur = state.stacks.get(&fk_key(slot)).copied().unwrap_or(0.0);
    if cur < max_stacks {
        if let Some(e) = state.enemies[slot].as_mut() { e.vulnerability += 7.0; }
        state.stacks.insert(fk_key(slot), cur + 1.0);
    }
    state.stacks.insert(fk_dur_key(slot), 3.0);
}

/// Remove all Firekiss stacks and revert vulnerability.
fn remove_firekiss(state: &mut SimState, slot: usize) {
    let fk = state.stacks.remove(&fk_key(slot)).unwrap_or(0.0);
    state.stacks.remove(&fk_dur_key(slot));
    if fk > 0.0 {
        if let Some(e) = state.enemies[slot].as_mut() { e.vulnerability -= 7.0 * fk; }
    }
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 120.0;

    // Minor traces: +22.4% Fire DMG (dmg_boost), +10% EHR, +24% BE
    state.team[idx].buffs.dmg_boost       += 22.4;
    state.team[idx].buffs.effect_hit_rate += 10.0;
    state.team[idx].buffs.break_effect    += 24.0;

    // A4: +25% action advance at battle start — NOT MODELED (AV-queue init timing)

    set(state, idx, BURN_ACC,   0.0);
    set(state, idx, A6_APPLIED, 0.0);
}

pub fn on_turn_start(_state: &mut SimState, _idx: usize) {}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    let t = target_idx.or_else(|| {
        state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
    });

    match action.action_type {
        ActionType::Basic => {
            action.multiplier       = 1.00;
            action.toughness_damage = 10.0;
        }
        ActionType::Skill => {
            // Main target only; adjacent hit handled in on_after_action
            action.multiplier       = 1.20;
            action.toughness_damage = 20.0;
        }
        _ => {}
    }

    // A6: +20% DMG if main target is already Burned. Manual revert in on_after_action.
    if let Some(t) = t {
        if is_burned(state, t) {
            state.team[idx].buffs.dmg_boost += 20.0;
            set(state, idx, A6_APPLIED, 1.0);
        } else {
            set(state, idx, A6_APPLIED, 0.0);
        }
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // Revert A6 dmg_boost applied in on_before_action
    if get(state, idx, A6_APPLIED) >= 1.0 {
        state.team[idx].buffs.dmg_boost -= 20.0;
        set(state, idx, A6_APPLIED, 0.0);
    }

    let t = match target_idx.or_else(|| {
        state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
    }) {
        Some(t) => t,
        None    => return,
    };

    let eidolon = state.team[idx].eidolon;

    match action.action_type {
        ActionType::Basic => {
            // E2: boost Burn DMG mult +40pp if target was ALREADY Burned on hit
            if eidolon >= 2 && is_burned(state, t) {
                state.stacks.insert(burn_boost_key(t), 40.0);
            }

            // A2: 80% base chance to Burn via accumulator
            let acc = get(state, idx, BURN_ACC) + 0.80;
            if acc >= 1.0 {
                apply_burn(state, t);
                set(state, idx, BURN_ACC, acc - 1.0);
                let name = state.team[idx].name.clone();
                state.add_log(&name, format!("A2: Burn applied to enemy {}", t));
            } else {
                set(state, idx, BURN_ACC, acc);
            }
        }

        ActionType::Skill => {
            // Snapshot pre-Burn state before applying new Burn
            let main_was_burned = is_burned(state, t);

            // Adjacent enemies (all other alive slots, up to 2)
            let adj: Vec<usize> = state.enemies.iter().enumerate()
                .filter(|(i, s)| *i != t && s.as_ref().map_or(false, |e| e.hp > 0.0))
                .map(|(i, _)| i)
                .take(2)
                .collect();

            // Adjacent hits: 40% ATK Fire DMG
            for &a in &adj {
                let adj_was_burned = is_burned(state, a);
                let adj_action = ActionParams {
                    action_type:      ActionType::Skill,
                    scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                    multiplier:       0.40,
                    extra_multiplier: 0.0,
                    extra_dmg:        0.0,
                    toughness_damage: 10.0,
                    inflicts_debuff:  false,
                    is_ult_dmg:       false,
                };
                let dmg = {
                    let mut m = state.team[idx].clone();
                    if adj_was_burned { m.buffs.dmg_boost += 20.0; } // A6 for adjacent
                    state.enemies[a].as_ref()
                        .map(|e| damage::calculate_damage(&m, e, &adj_action))
                        .unwrap_or(0.0)
                };
                if dmg > 0.0 {
                    if let Some(e) = state.enemies[a].as_mut() { e.hp -= dmg; }
                    state.total_damage += dmg;
                    if state.enemies[a].as_ref().map_or(false, |e| e.hp <= 0.0) {
                        state.enemies[a] = None;
                    }
                    let name = state.team[idx].name.clone();
                    state.add_log(&name, format!("Skill adj: {:.0} DMG (enemy {})", dmg, a));
                }
                // E2: adjacent target was already Burned on hit
                if eidolon >= 2 && adj_was_burned {
                    state.stacks.insert(burn_boost_key(a), 40.0);
                }
            }

            // E2: main target was already Burned on hit
            if eidolon >= 2 && main_was_burned {
                state.stacks.insert(burn_boost_key(t), 40.0);
            }

            // Apply Burn to main + adjacent (100% base chance → always)
            let mut count = 0usize;
            if state.enemies[t].as_ref().map_or(false, |e| e.hp > 0.0) {
                apply_burn(state, t);
                count += 1;
            }
            for &a in &adj {
                if state.enemies[a].as_ref().map_or(false, |e| e.hp > 0.0) {
                    apply_burn(state, a);
                    count += 1;
                }
            }
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!("Skill: Burn applied to {} target(s)", count));
        }

        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);

    // Restore 5 energy at ult entry BEFORE Burn procs add more
    state.team[idx].energy = 5.0;

    let name = state.team[idx].name.clone();
    let mut total_dmg = 0.0;
    let enemy_count = state.enemies.iter()
        .filter(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
        .count();

    for slot in 0..state.enemies.len() {
        if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }

        let burned = is_burned(state, slot);

        // AoE direct hit: 120% ATK (+ A6 if Burned)
        let ult_dmg = {
            let mut m = state.team[idx].clone();
            if burned { m.buffs.dmg_boost += 20.0; } // A6
            let ult_action = ActionParams {
                action_type:      ActionType::Ultimate,
                scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                multiplier:       1.20,
                extra_multiplier: 0.0,
                extra_dmg:        0.0,
                toughness_damage: 20.0,
                inflicts_debuff:  false,
                is_ult_dmg:       true,
            };
            state.enemies[slot].as_ref()
                .map(|e| damage::calculate_damage(&m, e, &ult_action))
                .unwrap_or(0.0)
        };
        if ult_dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= ult_dmg; }
            state.total_damage += ult_dmg;
            total_dmg += ult_dmg;
            if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[slot] = None;
            }
        }

        // Instant Burn proc: 92% × full Burn DoT if currently Burned
        if burned && state.enemies[slot].as_ref().map_or(false, |e| e.hp > 0.0) {
            let burn_boost = state.stacks.get(&burn_boost_key(slot)).copied().unwrap_or(0.0);
            let proc_mult  = (2.182 + burn_boost / 100.0) * 0.92;
            let proc_dmg   = burn_proc(state, idx, slot, proc_mult);
            total_dmg += proc_dmg;
            if proc_dmg > 0.0 {
                let name = state.team[idx].name.clone();
                state.add_log(&name, format!("Ult Burn proc: {:.0} DMG (enemy {})", proc_dmg, slot));
            }
        }
    }

    state.add_log(&name, format!(
        "Ult: {:.0} total DMG ({} targets, incl. Burn procs)",
        total_dmg, enemy_count,
    ));
}

pub fn on_enemy_turn_start(state: &mut SimState, idx: usize, enemy_idx: usize) {
    if state.enemies[enemy_idx].as_ref().map_or(true, |e| e.hp <= 0.0) { return; }

    // ── Burn DoT ──────────────────────────────────────────────────────────────
    let burn_turns = state.stacks.get(&burn_key(enemy_idx)).copied().unwrap_or(0.0);
    if burn_turns > 0.0 {
        let burn_boost = state.stacks.get(&burn_boost_key(enemy_idx)).copied().unwrap_or(0.0);
        let burn_mult  = 2.182 + burn_boost / 100.0;

        let dot_dmg = burn_proc(state, idx, enemy_idx, burn_mult);
        if dot_dmg > 0.0 {
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!(
                "Burn DoT: {:.0} DMG (enemy {}, {:.1}% ATK{})",
                dot_dmg, enemy_idx, burn_mult * 100.0,
                if burn_boost > 0.0 { " +E2" } else { "" },
            ));
        }

        // Tick Burn duration
        let new_turns = burn_turns - 1.0;
        if new_turns <= 0.0 {
            state.stacks.remove(&burn_key(enemy_idx));
            state.stacks.remove(&burn_boost_key(enemy_idx));
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!("Burn expired (enemy {})", enemy_idx));
        } else {
            state.stacks.insert(burn_key(enemy_idx), new_turns);
        }
    }

    // ── Firekiss duration tick ────────────────────────────────────────────────
    let fk_dur = state.stacks.get(&fk_dur_key(enemy_idx)).copied().unwrap_or(0.0);
    if fk_dur > 0.0 {
        let new_dur = fk_dur - 1.0;
        if new_dur <= 0.0 {
            remove_firekiss(state, enemy_idx);
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!("Firekiss expired (enemy {})", enemy_idx));
        } else {
            state.stacks.insert(fk_dur_key(enemy_idx), new_dur);
        }
    }
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
