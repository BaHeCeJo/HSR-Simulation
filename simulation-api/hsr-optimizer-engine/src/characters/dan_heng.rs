use crate::effects;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

// ─── Stack keys (static str → TeamMember.stacks) ─────────────────────────────
const RES_PEN_READY: &str = "dh_res_pen_ready";  // talent RES PEN primed flag
const TALENT_CD: &str     = "dh_talent_cd";       // turns until talent can trigger again
const A4_SPD_REM: &str    = "dh_a4_spd_rem";      // remaining turns of A4 SPD buff
const A4_FLIP: &str       = "dh_a4_flip";          // deterministic 50% flip for A4
const TECH_REM: &str      = "dh_tech_rem";         // remaining turns of technique ATK buff

// ─── A4 helper (50% chance +20% SPD, deterministic alternation) ──────────────
fn apply_a4(state: &mut SimState, idx: usize) {
    let flip = state.team[idx].stacks.get(A4_FLIP).copied().unwrap_or(0.0);
    state.team[idx].stacks.insert(A4_FLIP, if flip < 0.5 { 1.0 } else { 0.0 });
    if flip < 0.5 {
        let old_rem = state.team[idx].stacks.get(A4_SPD_REM).copied().unwrap_or(0.0);
        if old_rem <= 0.0 {
            state.team[idx].buffs.speed_percent += 20.0;
        }
        state.team[idx].stacks.insert(A4_SPD_REM, 2.0);
    }
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 100.0;
    state.team[idx].buffs.dmg_boost   += 22.4; // Wind DMG +22.4%
    state.team[idx].buffs.atk_percent += 18.0; // minor trace
    state.team[idx].buffs.def_percent += 12.5; // minor trace
    // Technique: +40% ATK for 3 turns at battle start
    state.team[idx].buffs.atk_percent += 40.0;
    state.team[idx].stacks.insert(TECH_REM, 3.0);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // Tick technique ATK buff
    let tech = state.team[idx].stacks.get(TECH_REM).copied().unwrap_or(0.0);
    if tech > 0.0 {
        if tech <= 1.0 {
            state.team[idx].stacks.remove(TECH_REM);
            state.team[idx].buffs.atk_percent -= 40.0;
        } else {
            state.team[idx].stacks.insert(TECH_REM, tech - 1.0);
        }
    }

    // Tick talent cooldown
    let cd = state.team[idx].stacks.get(TALENT_CD).copied().unwrap_or(0.0);
    if cd > 0.0 {
        if cd <= 1.0 {
            state.team[idx].stacks.remove(TALENT_CD);
        } else {
            state.team[idx].stacks.insert(TALENT_CD, cd - 1.0);
        }
    }

    // Tick A4 SPD buff
    let a4 = state.team[idx].stacks.get(A4_SPD_REM).copied().unwrap_or(0.0);
    if a4 > 0.0 {
        if a4 <= 1.0 {
            state.team[idx].stacks.remove(A4_SPD_REM);
            state.team[idx].buffs.speed_percent -= 20.0;
        } else {
            state.team[idx].stacks.insert(A4_SPD_REM, a4 - 1.0);
        }
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    // Consume Talent RES PEN charge if primed (auto-reverts via buffs snapshot)
    if state.team[idx].stacks.remove(RES_PEN_READY).is_some() {
        state.team[idx].buffs.res_pen += 36.0;
    }

    let eidolon = state.team[idx].eidolon;

    // E1: +12% CRIT Rate when target HP >= 50%
    if eidolon >= 1 {
        let hp_pct = target_idx
            .and_then(|t| state.enemies.get(t)).and_then(|e| e.as_ref())
            .map_or(1.0, |e| if e.max_hp > 0.0 { e.hp / e.max_hp } else { 1.0 });
        if hp_pct >= 0.5 {
            state.team[idx].buffs.crit_rate += 12.0;
        }
    }

    let is_slowed = target_idx
        .and_then(|t| state.enemies.get(t)).and_then(|e| e.as_ref())
        .map_or(false, |e| e.active_debuffs.contains_key("dan_heng_slow"));

    match action.action_type {
        ActionType::Basic => {
            action.multiplier       = 1.00;
            action.toughness_damage = 10.0;
            // A6: Basic ATK deals 40% more DMG to Slowed enemies
            if is_slowed {
                state.team[idx].buffs.basic_atk_dmg_boost += 40.0;
            }
        }
        ActionType::Skill => {
            action.multiplier       = 2.60;
            action.toughness_damage = 20.0;
            action.inflicts_debuff  = true;
        }
        ActionType::Ultimate => {
            action.multiplier       = 4.00;
            action.toughness_damage = 30.0;
            // +120% multiplier bonus if target is Slowed
            if is_slowed {
                action.extra_multiplier += 120.0;
            }
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
    let eidolon = state.team[idx].eidolon;

    // Skill: apply Slow (treated as guaranteed CRIT proc, 100% base chance)
    if action.action_type == ActionType::Skill {
        if let Some(t) = target_idx {
            if let Some(enemy) = state.enemies.get_mut(t).and_then(|e| e.as_mut()) {
                if enemy.hp > 0.0 {
                    // E6: Slow reduces SPD by extra 8% → 20% total; base 12%
                    let slow_val = if eidolon >= 6 { 20.0 } else { 12.0 };
                    effects::apply_enemy_debuff(enemy, "dan_heng_slow", StatusEffect {
                        duration: 2,
                        value:    slow_val,
                        stat:     None,
                        effects:  vec![],
                    });
                }
            }
        }
    }

    // E4: ult kill → next action Advanced Forward ~100%
    if eidolon >= 4 && action.action_type == ActionType::Ultimate {
        if let Some(t) = target_idx {
            if state.enemies.get(t).map_or(false, |e| e.is_none()) {
                // Encode 100% advance as 99.9 (effective_spd clamps at < 100)
                state.team[idx].stacks.insert("_action_advance_pct", 99.9);
            }
        }
    }

    // A4: 50% chance +20% SPD for 2 turns after any attack
    match action.action_type {
        ActionType::Basic | ActionType::Skill | ActionType::Ultimate => apply_a4(state, idx),
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    // Default single-target damage path is used (no _ult_handled).
    // Slowed bonus is handled in on_before_action via extra_multiplier.
    state.team[idx].energy = 5.0; // residue energy after ult
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,
    _source_idx: usize,
    action: &ActionParams,
    _target_idx: Option<usize>,
) {
    // Talent: when an ally uses a Skill (approximation for "targeted by ally Ability"),
    // prime Wind RES PEN +36% for Dan Heng's next attack if not on cooldown.
    if action.action_type == ActionType::Skill {
        let cd = state.team[idx].stacks.get(TALENT_CD).copied().unwrap_or(0.0);
        if cd <= 0.0 {
            state.team[idx].stacks.insert(RES_PEN_READY, 1.0);
            let new_cd = if state.team[idx].eidolon >= 2 { 1.0 } else { 2.0 };
            state.team[idx].stacks.insert(TALENT_CD, new_cd);
        }
    }
}
