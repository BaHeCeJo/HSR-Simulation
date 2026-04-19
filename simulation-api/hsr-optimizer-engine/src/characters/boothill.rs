//! Boothill  (The Hunt | Physical)
//!
//! Skill: Enter Standoff with target (2 turns of Enhanced Basic). No direct DMG.
//! Enhanced Basic (Standoff): 220% ATK, toughness 20 + 10×Trickshot, +30% DMG.
//! Ult: 400% ATK single target, implant Physical Weakness (2 enemy turns).
//! Talent: On Weakness Break → Talent Break DMG + gain Trickshot + end Standoff.
//! A2: In Standoff, if CRIT Rate ≥ 50% → +15% CR and +15% CD.
//! A6: +10 energy per Trickshot gained.
//! Minor Traces: BE +37.3%, ATK +18%, HP +10%.
//! E1: Start with 1 Trickshot. E2: Trickshot gain (once/turn) → +1 SP, +30% BE.
//! E4: +12% DMG in Standoff. E6: On break, extra element break DMG.

use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

const STANDOFF_KEY:    &str = "boothill_standoff";     // remaining Standoff turns (0 = none)
const STANDOFF_TARGET: &str = "boothill_standoff_tgt"; // enemy slot as f64
const TRICKSHOT_KEY:   &str = "boothill_trickshot";    // 0-3 Trickshot stacks
const ENH_FLAG:        &str = "boothill_enh";           // Enhanced Basic active this turn
const ENTERING_KEY:    &str = "boothill_entering";      // Skill → inline ENH Basic
const E2_GUARD:        &str = "boothill_e2g";           // once-per-turn E2 gate

fn ts(state: &SimState, idx: usize) -> f64 {
    state.team[idx].stacks.get(TRICKSHOT_KEY).copied().unwrap_or(0.0)
}

fn in_standoff(state: &SimState, idx: usize) -> bool {
    state.team[idx].stacks.get(STANDOFF_KEY).copied().unwrap_or(0.0) > 0.0
}

fn gain_trickshot(state: &mut SimState, idx: usize) {
    let cur = ts(state, idx);
    if cur >= 3.0 { return; }
    state.team[idx].stacks.insert(TRICKSHOT_KEY.to_string(), cur + 1.0);

    // A6: +10 energy per Trickshot gained
    let max_e = state.team[idx].max_energy;
    state.team[idx].energy = (state.team[idx].energy + 10.0).min(max_e);

    // E2: once per turn — +1 SP and permanently +30% BE
    if state.team[idx].eidolon >= 2 {
        let guard = state.team[idx].stacks.get(E2_GUARD).copied().unwrap_or(0.0);
        if guard == 0.0 {
            state.team[idx].stacks.insert(E2_GUARD.to_string(), 1.0);
            state.skill_points = (state.skill_points + 1).min(5);
            let old = state.team[idx].base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0);
            state.team[idx].base_stats.insert(ids::CHAR_BE_ID.to_string(), old + 30.0);
        }
    }
}

fn dispel_standoff(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert(STANDOFF_KEY.to_string(), 0.0);
    state.team[idx].stacks.remove(STANDOFF_TARGET);
}

/// Boothill Talent Break DMG: pct × Physical Break DMG (max_toughness capped at 160).
pub fn talent_break_dmg(state: &mut SimState, idx: usize, enemy_slot: usize) {
    let stacks = ts(state, idx);
    let pct = if stacks <= 0.0 { 0.70 } else if stacks < 2.0 { 1.20 } else { 1.70 };

    let attacker = state.team[idx].clone();
    let break_dmg = if let Some(enemy) = state.enemies[enemy_slot].as_ref() {
        let mut fake = enemy.clone();
        fake.max_toughness = enemy.max_toughness.min(160.0);
        (damage::calculate_break_damage(&attacker, &fake) * pct).floor()
    } else {
        0.0
    };

    if break_dmg > 0.0 {
        if let Some(e) = state.enemies[enemy_slot].as_mut() { e.hp -= break_dmg; }
        state.total_damage += break_dmg;
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Talent Break ({:.0}% Physical Break): {:.0} DMG", pct * 100.0, break_dmg));
    }
}

/// Execute Enhanced Basic inline (after Skill entry), handling damage + toughness manually.
fn execute_inline_enhanced_basic(state: &mut SimState, idx: usize) {
    let target_slot = state.team[idx].stacks.get(STANDOFF_TARGET).copied().unwrap_or(0.0) as usize;

    if state.enemies.get(target_slot).and_then(|s| s.as_ref()).map_or(true, |e| e.hp <= 0.0) {
        gain_trickshot(state, idx);
        dispel_standoff(state, idx);
        return;
    }

    let stacks = ts(state, idx);
    let toughness_dmg = 20.0 + stacks * 10.0;

    // Build buffs-augmented member snapshot for damage calculation
    let mut member = state.team[idx].clone();
    member.buffs.dmg_boost += 30.0; // A4: Standoff +30% DMG
    if member.eidolon >= 4 { member.buffs.dmg_boost += 12.0; } // E4
    if member.eidolon >= 1 { member.buffs.def_ignore += 16.0; } // E1
    // A2: CRIT bonuses if CRIT Rate >= 50%
    let base_cr = member.base_stats.get(ids::CHAR_CR_ID).copied().unwrap_or(0.0);
    if base_cr + member.buffs.crit_rate >= 50.0 {
        member.buffs.crit_rate += 15.0;
        member.buffs.crit_dmg  += 15.0;
    }

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

    // Toughness reduction — only if Physical weakness
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
        gain_trickshot(state, idx);
        dispel_standoff(state, idx);
        state.team[idx].stacks.insert(BROKE_FLAG.to_string(), 1.0);
    } else if state.enemies[target_slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
        // Killed without break
        gain_trickshot(state, idx);
        dispel_standoff(state, idx);
    }

    if state.enemies.get(target_slot).and_then(|s| s.as_ref()).map_or(false, |e| e.hp <= 0.0) {
        state.enemies[target_slot] = None;
    }
}

// ─── Public hooks ─────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy         = 115.0;
    // Minor traces
    let old_be = state.team[idx].base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0);
    state.team[idx].base_stats.insert(ids::CHAR_BE_ID.to_string(), old_be + 37.3);
    state.team[idx].buffs.atk_percent += 18.0;
    state.team[idx].buffs.hp_percent  += 10.0;
    // Init stacks
    state.team[idx].stacks.insert(STANDOFF_KEY.to_string(), 0.0);
    state.team[idx].stacks.insert(TRICKSHOT_KEY.to_string(), 0.0);
    // E1: start with 1 Trickshot (bypass gain_trickshot to avoid E2 at battle start)
    if state.team[idx].eidolon >= 1 {
        state.team[idx].stacks.insert(TRICKSHOT_KEY.to_string(), 1.0);
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert(E2_GUARD.to_string(), 0.0);
    state.team[idx].stacks.insert(BROKE_FLAG.to_string(), 0.0);
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Skill => {
            // Enter Standoff — zero out Skill damage, execute Enhanced Basic inline later
            action.multiplier       = 0.0;
            action.toughness_damage = 0.0;
            let slot = target_idx.unwrap_or(0) as f64;
            state.team[idx].stacks.insert(STANDOFF_KEY.to_string(), 2.0);
            state.team[idx].stacks.insert(STANDOFF_TARGET.to_string(), slot);
            state.team[idx].stacks.insert(ENTERING_KEY.to_string(), 1.0);
        }
        ActionType::Basic if in_standoff(state, idx) => {
            let stacks = ts(state, idx);
            action.scaling_stat_id  = ids::CHAR_ATK_ID.to_string();
            action.multiplier       = 2.20;
            action.toughness_damage = 20.0 + stacks * 10.0;
            state.team[idx].stacks.insert(ENH_FLAG.to_string(), 1.0);
            // A4: +30% DMG in Standoff
            state.team[idx].buffs.dmg_boost += 30.0;
            // E4: +12% DMG in Standoff
            if state.team[idx].eidolon >= 4 { state.team[idx].buffs.dmg_boost += 12.0; }
            // E1: +16% DEF ignore in Standoff
            if state.team[idx].eidolon >= 1 { state.team[idx].buffs.def_ignore += 16.0; }
            // A2: CRIT bonuses when CRIT Rate >= 50%
            let base_cr = state.team[idx].base_stats.get(ids::CHAR_CR_ID).copied().unwrap_or(0.0);
            if base_cr + state.team[idx].buffs.crit_rate >= 50.0 {
                state.team[idx].buffs.crit_rate += 15.0;
                state.team[idx].buffs.crit_dmg  += 15.0;
            }
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
        state.team[idx].stacks.insert(ENTERING_KEY.to_string(), 0.0);
        // Correct energy: Skill gave +30*err, Enhanced Basic gives +20*err
        let err_mult = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
        state.team[idx].energy -= 30.0 * err_mult;
        state.team[idx].energy += 20.0 * err_mult;
        execute_inline_enhanced_basic(state, idx);
        return;
    }

    // ── Enhanced Basic (normal Standoff turn) ─────────────────────────────────
    if enh > 0.0 {
        state.team[idx].stacks.insert(ENH_FLAG.to_string(), 0.0);
        // Enhanced Basic doesn't grant SP (undo simulator's +1 for Basic ATK)
        state.skill_points = (state.skill_points - 1).max(0);

        // Save Standoff target before potentially removing it
        let standoff_t_f = state.team[idx].stacks.get(STANDOFF_TARGET).copied();

        // Decrement Standoff
        let remaining = state.team[idx].stacks.get(STANDOFF_KEY).copied().unwrap_or(0.0);
        let new_rem   = (remaining - 1.0).max(0.0);
        state.team[idx].stacks.insert(STANDOFF_KEY.to_string(), new_rem);
        if new_rem <= 0.0 {
            state.team[idx].stacks.remove(STANDOFF_TARGET);
        }

        // Check if Standoff target was killed without break → gain Trickshot
        if let (Some(t), Some(st_f)) = (target_idx, standoff_t_f) {
            if t == st_f as usize && state.enemies.get(t).map_or(false, |s| s.is_none()) {
                let broke = state.team[idx].stacks.get(BROKE_FLAG).copied().unwrap_or(0.0);
                if broke == 0.0 {
                    gain_trickshot(state, idx);
                }
            }
        }
        state.team[idx].stacks.insert(BROKE_FLAG.to_string(), 0.0);
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
            // Track Physical Weakness duration (2 enemy turns)
            let dur_key = format!("boothill_phys_dur_{}", t);
            state.team[idx].stacks.insert(dur_key, 2.0);
        }
    }
}

pub fn on_ult(_state: &mut SimState, _idx: usize) {
    // No custom ult needed — simulator handles default single-target damage.
    // Multiplier (4.00) is set in on_before_action.
}

pub fn on_break(state: &mut SimState, idx: usize, enemy_idx: usize) {
    // Talent: Talent Break DMG, gain Trickshot, end Standoff
    talent_break_dmg(state, idx, enemy_idx);
    gain_trickshot(state, idx);
    dispel_standoff(state, idx);
    state.team[idx].stacks.insert(BROKE_FLAG.to_string(), 1.0);

    // E6: additional element break DMG (~40% more Physical Break equivalent)
    if state.team[idx].eidolon >= 6 {
        let attacker = state.team[idx].clone();
        let extra = if let Some(enemy) = state.enemies[enemy_idx].as_ref() {
            let mut fake = enemy.clone();
            fake.max_toughness = enemy.max_toughness.min(160.0);
            damage::calculate_break_damage(&attacker, &fake) * 0.40
        } else { 0.0 };
        if extra > 0.0 {
            if let Some(e) = state.enemies[enemy_idx].as_mut() { e.hp -= extra; }
            state.total_damage += extra;
        }
    }
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(state: &mut SimState, idx: usize, enemy_idx: usize) {
    // Decrement Physical Weakness duration for this enemy slot
    let dur_key = format!("boothill_phys_dur_{}", enemy_idx);
    let dur = state.team[idx].stacks.get(&dur_key).copied().unwrap_or(0.0);
    if dur <= 0.0 { return; }
    let new_dur = dur - 1.0;
    if new_dur <= 0.0 {
        state.team[idx].stacks.remove(&dur_key);
        if let Some(enemy) = state.enemies[enemy_idx].as_mut() {
            enemy.weaknesses.retain(|w| w != "Physical");
        }
    } else {
        state.team[idx].stacks.insert(dur_key, new_dur);
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
