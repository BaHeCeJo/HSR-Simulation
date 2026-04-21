//! Boothill  (The Hunt | Physical)
//!
//! Skill: Enter Standoff (2 ENH Basic turns, turn doesn't end). No direct DMG.
//! Enhanced Basic: 220% ATK, toughness 20+50%×Trickshot per stack, talent Break on break.
//! Ult: 400% ATK, implant Physical Weakness 2 enemy-turns, +5 energy leftover.
//! Talent: On Weakness Break during ENH Basic → Break DMG (1ts=70%/2ts=120%/3ts=170%) + Standoff ends.
//! A2 Ghost Load: CR = min(BE×10%, 30%), CD = min(BE×50%, 150%).
//! A4 Above Snakes: -30% incoming DMG from non-Standoff enemies (defensive, not simulated).
//! A6 Point Blank: +10 energy when in Standoff and gaining Trickshot (incl. beyond cap).
//! E1: +1 Trickshot at battle start; +16% DEF ignore on DMG.
//! E2: On Trickshot gain in Standoff (once/turn): +1 SP, +30% BE for 2 turns (incl. beyond cap).
//! E4: Standoff target receives +12% DMG from Boothill.
//! E6: Talent Break also deals 40% to same target + 70% to adjacent targets.
//! Minor Traces: BE +37.3%, ATK +18%, HP +10%.

use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

const STANDOFF_KEY:    &str = "boothill_standoff";     // remaining Standoff turns (0 = none)
const STANDOFF_TARGET: &str = "boothill_standoff_tgt"; // enemy slot index as f64
const TRICKSHOT_KEY:   &str = "boothill_trickshot";    // 0–3 Pocket Trickshot stacks
const ENH_FLAG:        &str = "boothill_enh";           // Enhanced Basic active this turn
const ENTERING_KEY:    &str = "boothill_entering";      // Skill → inline ENH Basic
const E2_GUARD:        &str = "boothill_e2g";           // once-per-turn E2 gate
const E2_BE_DUR:       &str = "boothill_e2_be_dur";    // turns remaining on E2 BE bonus

fn ts(state: &SimState, idx: usize) -> f64 {
    state.team[idx].stacks.get(TRICKSHOT_KEY).copied().unwrap_or(0.0)
}

fn in_standoff(state: &SimState, idx: usize) -> bool {
    state.team[idx].stacks.get(STANDOFF_KEY).copied().unwrap_or(0.0) > 0.0
}

/// Apply E2's +30% BE timed bonus to base_stats. Refreshes duration without stacking.
fn e2_apply_be_bonus(state: &mut SimState, idx: usize) {
    let dur = state.team[idx].stacks.get(E2_BE_DUR).copied().unwrap_or(0.0);
    if dur <= 0.0 {
        let old = state.team[idx].base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0);
        state.team[idx].base_stats.insert(ids::CHAR_BE_ID.to_string(), old + 30.0);
    }
    state.team[idx].stacks.insert(E2_BE_DUR, 2.0);
}

/// Called whenever Boothill gains (or would gain) a Trickshot stack.
/// A6 energy and E2 effects trigger even when already at 3 stacks.
fn gain_trickshot(state: &mut SimState, idx: usize) {
    // A6: +10 energy per Trickshot gained while in Standoff (including beyond cap)
    if in_standoff(state, idx) {
        let max_e = state.team[idx].max_energy;
        state.team[idx].energy = (state.team[idx].energy + 10.0).min(max_e);
    }

    // E2: once per turn — +1 SP and +30% BE for 2 turns (including beyond cap, only in Standoff)
    if state.team[idx].eidolon >= 2 && in_standoff(state, idx) {
        let guard = state.team[idx].stacks.get(E2_GUARD).copied().unwrap_or(0.0);
        if guard == 0.0 {
            state.team[idx].stacks.insert(E2_GUARD, 1.0);
            state.skill_points = (state.skill_points + 1).min(5);
            e2_apply_be_bonus(state, idx);
        }
    }

    let cur = ts(state, idx);
    if cur < 3.0 {
        state.team[idx].stacks.insert(TRICKSHOT_KEY, cur + 1.0);
    }
}

fn dispel_standoff(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert(STANDOFF_KEY, 0.0);
    state.team[idx].stacks.remove(STANDOFF_TARGET);
}

/// A2 Ghost Load: apply BE-scaled CR/CD to buffs (temporary, reverts after action).
fn apply_a2_cr_cd(state: &mut SimState, idx: usize) {
    let be = state.team[idx].base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0);
    state.team[idx].buffs.crit_rate += (be * 0.10_f64).min(30.0);
    state.team[idx].buffs.crit_dmg  += (be * 0.50_f64).min(150.0);
}

/// Talent Break DMG: 1 Trickshot=70%, 2=120%, 3=170% of Physical Break DMG.
/// Max toughness for this DMG is capped at 16× base Basic ATK toughness (160).
/// E6: additionally 40% to same target + 70% to adjacent targets.
pub fn talent_break_dmg(state: &mut SimState, idx: usize, enemy_slot: usize) {
    let stacks = ts(state, idx);
    let pct = if stacks <= 1.0 { 0.70 } else if stacks <= 2.0 { 1.20 } else { 1.70 };

    let attacker = state.team[idx].clone();

    let break_dmg = if let Some(enemy) = state.enemies[enemy_slot].as_ref() {
        let mut fake = enemy.clone();
        fake.max_toughness = enemy.max_toughness.min(160.0);
        (damage::calculate_break_damage(&attacker, &fake) * pct).floor()
    } else { 0.0 };

    if break_dmg > 0.0 {
        if let Some(e) = state.enemies[enemy_slot].as_mut() { e.hp -= break_dmg; }
        state.total_damage += break_dmg;
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Talent Break ({:.0}% Physical Break): {:.0} DMG", pct * 100.0, break_dmg));
    }

    // E6: 40% to same target + 70% to adjacent targets
    if state.team[idx].eidolon >= 6 {
        let e6_same = if let Some(enemy) = state.enemies[enemy_slot].as_ref() {
            let mut fake = enemy.clone();
            fake.max_toughness = enemy.max_toughness.min(160.0);
            (damage::calculate_break_damage(&attacker, &fake) * pct * 0.40).floor()
        } else { 0.0 };
        if e6_same > 0.0 {
            if let Some(e) = state.enemies[enemy_slot].as_mut() { e.hp -= e6_same; }
            state.total_damage += e6_same;
        }

        let n_enemies = state.enemies.len();
        for slot in 0..n_enemies {
            if slot == enemy_slot { continue; }
            let adj_dmg = if let Some(enemy) = state.enemies[slot].as_ref() {
                let mut fake = enemy.clone();
                fake.max_toughness = enemy.max_toughness.min(160.0);
                (damage::calculate_break_damage(&attacker, &fake) * pct * 0.70).floor()
            } else { 0.0 };
            if adj_dmg > 0.0 {
                if let Some(e) = state.enemies[slot].as_mut() { e.hp -= adj_dmg; }
                state.total_damage += adj_dmg;
            }
        }
    }
}

/// Inline Enhanced Basic executed after Skill entry (turn doesn't end).
fn execute_inline_enhanced_basic(state: &mut SimState, idx: usize) {
    let target_slot = state.team[idx].stacks.get(STANDOFF_TARGET).copied().unwrap_or(0.0) as usize;

    // Target already dead: Skill passive (defeated) → Trickshot + end Standoff
    if state.enemies.get(target_slot).and_then(|s| s.as_ref()).map_or(true, |e| e.hp <= 0.0) {
        gain_trickshot(state, idx);  // Skill passive: defeated
        dispel_standoff(state, idx);
        return;
    }

    let stacks = ts(state, idx);
    let toughness_dmg = 20.0 + stacks * 10.0;

    // Build augmented snapshot for damage calculation
    let mut member = state.team[idx].clone();
    member.buffs.dmg_boost += 30.0;                                    // Standoff +30% DMG
    if member.eidolon >= 4 { member.buffs.dmg_boost += 12.0; }        // E4
    if member.eidolon >= 1 { member.buffs.def_ignore += 16.0; }       // E1
    // A2: BE-scaled CR/CD
    let be = member.base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0);
    member.buffs.crit_rate += (be * 0.10_f64).min(30.0);
    member.buffs.crit_dmg  += (be * 0.50_f64).min(150.0);

    let enh_action = ActionParams {
        action_type:      ActionType::Basic,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       2.20,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: toughness_dmg,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let dmg = state.enemies[target_slot].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &enh_action))
        .unwrap_or(0.0);

    if dmg > 0.0 {
        if let Some(e) = state.enemies[target_slot].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Enhanced Basic (Standoff entry): {:.0} DMG", dmg));

    // Toughness reduction — only if enemy has Physical weakness
    let has_phys = state.enemies[target_slot].as_ref()
        .map_or(false, |e| e.weaknesses.contains(&"Physical".to_string()));

    let broke = if has_phys {
        if let Some(e) = state.enemies[target_slot].as_mut() {
            e.toughness = (e.toughness - toughness_dmg).max(0.0);
            if e.toughness <= 0.0 && !e.is_broken {
                e.is_broken = true;
                true
            } else { false }
        } else { false }
    } else { false };

    if broke {
        let attacker = state.team[idx].clone();
        let break_dmg = state.enemies[target_slot].as_ref()
            .map(|e| damage::calculate_break_damage(&attacker, e))
            .unwrap_or(0.0);
        if break_dmg > 0.0 {
            if let Some(e) = state.enemies[target_slot].as_mut() { e.hp -= break_dmg; }
            state.total_damage += break_dmg;
        }
        talent_break_dmg(state, idx, target_slot);
        gain_trickshot(state, idx);  // +1 for Weakness Break
    }

    // Kill grant: independent of break — fires if enemy died from hit or break damage
    let enemy_dead = state.enemies[target_slot].as_ref().map_or(false, |e| e.hp <= 0.0);
    if enemy_dead {
        gain_trickshot(state, idx);  // +1 for kill
    }

    // Dispel Standoff if break or kill ended it
    if broke || enemy_dead {
        dispel_standoff(state, idx);
    }

    if enemy_dead {
        state.enemies[target_slot] = None;
    }
}

// ─── Public hooks ─────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 115.0;
    // Minor traces
    let old_be = state.team[idx].base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0);
    state.team[idx].base_stats.insert(ids::CHAR_BE_ID.to_string(), old_be + 37.3);
    state.team[idx].buffs.atk_percent += 18.0;
    state.team[idx].buffs.hp_percent  += 10.0;
    // Init stacks
    state.team[idx].stacks.insert(STANDOFF_KEY, 0.0);
    state.team[idx].stacks.insert(TRICKSHOT_KEY, 0.0);
    // E1: start with 1 Trickshot — bypass gain_trickshot to avoid A6/E2 at battle start
    if state.team[idx].eidolon >= 1 {
        state.team[idx].stacks.insert(TRICKSHOT_KEY, 1.0);
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert(E2_GUARD, 0.0);

    // E2 BE bonus: decrement duration and remove when expired
    let dur = state.team[idx].stacks.get(E2_BE_DUR).copied().unwrap_or(0.0);
    if dur > 0.0 {
        let new_dur = dur - 1.0;
        state.team[idx].stacks.insert(E2_BE_DUR, new_dur);
        if new_dur <= 0.0 {
            let old = state.team[idx].base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0);
            state.team[idx].base_stats.insert(ids::CHAR_BE_ID.to_string(), (old - 30.0).max(0.0));
        }
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Skill => {
            // Enter Standoff — zero out Skill damage, execute Enhanced Basic inline in on_after_action
            action.multiplier       = 0.0;
            action.toughness_damage = 0.0;
            let slot = target_idx.unwrap_or(0) as f64;
            state.team[idx].stacks.insert(STANDOFF_KEY, 2.0);
            state.team[idx].stacks.insert(STANDOFF_TARGET, slot);
            state.team[idx].stacks.insert(ENTERING_KEY, 1.0);
        }
        ActionType::Basic if in_standoff(state, idx) => {
            let stacks = ts(state, idx);
            action.scaling_stat_id  = ids::CHAR_ATK_ID.to_string();
            action.multiplier       = 2.20;
            action.toughness_damage = 20.0 + stacks * 10.0;
            state.team[idx].stacks.insert(ENH_FLAG, 1.0);
            // Standoff DMG bonuses (revert after action via snapshot)
            state.team[idx].buffs.dmg_boost += 30.0;
            if state.team[idx].eidolon >= 4 { state.team[idx].buffs.dmg_boost += 12.0; }
            if state.team[idx].eidolon >= 1 { state.team[idx].buffs.def_ignore += 16.0; }
            // A2: BE-scaled CR/CD
            apply_a2_cr_cd(state, idx);
        }
        ActionType::Basic => {
            action.scaling_stat_id  = ids::CHAR_ATK_ID.to_string();
            action.multiplier       = 1.10;
            action.toughness_damage = 10.0;
        }
        ActionType::Ultimate => {
            action.scaling_stat_id  = ids::CHAR_ATK_ID.to_string();
            action.multiplier       = 4.00;
            action.toughness_damage = 30.0;
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
    let entering = state.team[idx].stacks.get(ENTERING_KEY).copied().unwrap_or(0.0);
    let enh      = state.team[idx].stacks.get(ENH_FLAG).copied().unwrap_or(0.0);

    // ── Skill → Standoff entry: execute Enhanced Basic inline ─────────────────
    if entering > 0.0 {
        state.team[idx].stacks.insert(ENTERING_KEY, 0.0);
        // Correct energy: Skill gave +30×ERR, Enhanced Basic gives +20×ERR
        let err_mult = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
        state.team[idx].energy -= 30.0 * err_mult;
        state.team[idx].energy += 20.0 * err_mult;
        execute_inline_enhanced_basic(state, idx);
        return;
    }

    // ── Enhanced Basic (normal Standoff turn) ─────────────────────────────────
    if enh > 0.0 {
        state.team[idx].stacks.insert(ENH_FLAG, 0.0);
        // Enhanced Basic doesn't grant SP (undo simulator's +1 for Basic ATK)
        state.skill_points = (state.skill_points - 1).max(0);

        // Decrement Standoff turns (may already be 0 if on_break called dispel_standoff)
        let remaining = state.team[idx].stacks.get(STANDOFF_KEY).copied().unwrap_or(0.0);
        let new_rem   = (remaining - 1.0).max(0.0);
        state.team[idx].stacks.insert(STANDOFF_KEY, new_rem);
        if new_rem <= 0.0 {
            state.team[idx].stacks.remove(STANDOFF_TARGET);
        }

        // Kill grant: ENH Basic can only target the Standoff enemy, so target_idx is always
        // the Standoff target. Use it directly so this fires whether or not a break happened.
        if let Some(t) = target_idx {
            if state.enemies.get(t).map_or(false, |s| s.is_none()) {
                gain_trickshot(state, idx);  // +1 for kill
            }
        }
        return;
    }

    // ── Ultimate ───────────────────────────────────────────────────────────────
    if matches!(action.action_type, ActionType::Ultimate) {
        state.team[idx].energy = 5.0;
        if let Some(t) = target_idx {
            if let Some(enemy) = state.enemies.get_mut(t).and_then(|s| s.as_mut()) {
                if !enemy.weaknesses.contains(&"Physical".to_string()) {
                    enemy.weaknesses.push("Physical".to_string());
                }
            }
            let dur_key = format!("boothill_phys_dur_{}", t);
            state.stacks.insert(dur_key, 2.0);
        }
    }
}

pub fn on_ult(_state: &mut SimState, _idx: usize) {}

pub fn on_break(state: &mut SimState, idx: usize, enemy_idx: usize) {
    // Only fire Talent when Boothill is in Standoff with this specific enemy.
    // dispatch_on_break loops over ALL team members, so guard against other characters breaking.
    if !in_standoff(state, idx) { return; }
    let standoff_t = state.team[idx].stacks.get(STANDOFF_TARGET).copied().unwrap_or(-1.0) as usize;
    if standoff_t != enemy_idx { return; }

    talent_break_dmg(state, idx, enemy_idx);
    gain_trickshot(state, idx);  // +1 for Weakness Break
    dispel_standoff(state, idx);
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(state: &mut SimState, _idx: usize, enemy_idx: usize) {
    // Decrement Physical Weakness duration implanted by Ult
    let dur_key = format!("boothill_phys_dur_{}", enemy_idx);
    let dur = state.stacks.get(&dur_key).copied().unwrap_or(0.0);
    if dur <= 0.0 { return; }
    let new_dur = dur - 1.0;
    if new_dur <= 0.0 {
        state.stacks.remove(&dur_key);
        if let Some(enemy) = state.enemies[enemy_idx].as_mut() {
            enemy.weaknesses.retain(|w| w != "Physical");
        }
    } else {
        state.stacks.insert(dur_key, new_dur);
    }
}

pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
