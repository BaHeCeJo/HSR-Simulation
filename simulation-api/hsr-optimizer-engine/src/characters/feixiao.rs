use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ─── Stack keys (all in TeamMember.stacks, never snapshotted) ─────────────────
const FA:          &str = "feixiao_fa";       // Flying Aureus (0–12)
const FA_ACC:      &str = "feixiao_fa_acc";   // 0.5-accumulator for non-FUA attacks → FA
const TALENT_USED: &str = "feixiao_t_used";   // 1 if Talent FUA fired this Feixiao-turn
const E2_FUA_CT:   &str = "feixiao_e2_fct";   // E2 FUA FA grants this turn (cap 6)
// Timed buffs: REM = remaining Feixiao-turns; APPLD = 1 if currently in buffs
const TDMG_REM:    &str = "feixiao_tdmg_r";   // Talent +60% DMG remaining
const TDMG_APPLD:  &str = "feixiao_tdmg_a";   // +60% applied to buffs
const A6_REM:      &str = "feixiao_a6_r";     // A6 +48% ATK remaining
const A6_APPLD:    &str = "feixiao_a6_a";     // +48% applied to buffs
const E4_SPD_REM:  &str = "feixiao_e4_sr";    // E4 +8% SPD remaining
const E4_SPD_APPLD:&str = "feixiao_e4_sa";    // +8% SPD applied to buffs

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn first_alive(state: &SimState) -> Option<usize> {
    state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
}

fn get(state: &SimState, idx: usize, key: &str) -> f64 {
    state.team[idx].stacks.get(key).copied().unwrap_or(0.0)
}

fn set(state: &mut SimState, idx: usize, key: &'static str, v: f64) {
    state.team[idx].stacks.insert(key, v);
}

/// Add FA, capped at 12.
fn add_fa(state: &mut SimState, idx: usize, amount: f64) {
    let fa = get(state, idx, FA);
    set(state, idx, FA, (fa + amount).min(12.0));
}

/// Tick a timed buff (in on_turn_start).
/// Returns (should_apply, should_remove).
fn tick_timed(state: &mut SimState, idx: usize, rem_key: &'static str, appld_key: &'static str) -> (bool, bool) {
    let rem   = get(state, idx, rem_key);
    let appld = get(state, idx, appld_key);

    if rem <= 0.0 { return (false, false); }

    if appld < 1.0 {
        // First application turn — apply but don't tick yet
        set(state, idx, appld_key, 1.0);
        (true, false)
    } else {
        // Already applied — tick
        let new_rem = rem - 1.0;
        set(state, idx, rem_key, new_rem);
        if new_rem <= 0.0 {
            set(state, idx, appld_key, 0.0);
            (false, true)  // should remove
        } else {
            (false, false)
        }
    }
}

/// Build a clone of the member augmented for a Talent FUA hit.
fn fua_member_clone(state: &SimState, idx: usize) -> crate::models::TeamMember {
    let mut m = state.team[idx].clone();
    // A4: FUA CRIT DMG +36%
    m.buffs.crit_dmg += 36.0;
    // Talent +60% DMG: if not yet in actual buffs (TDMG_APPLD=0), add to clone manually
    if get(state, idx, TDMG_APPLD) < 1.0 {
        m.buffs.dmg_boost += 60.0;
    }
    let eidolon = state.team[idx].eidolon;
    if eidolon >= 6 {
        // E6: Talent FUA = Ult DMG → add ult_dmg_boost into dmg_boost; +20% RES PEN
        let ub = m.buffs.ult_dmg_boost;
        m.buffs.dmg_boost += ub;
        m.buffs.res_pen   += 20.0;
    }
    m
}

/// Fire one Talent FUA against `target_slot` (redirects if dead).
fn fire_talent_fua(state: &mut SimState, idx: usize, target_slot: usize) {
    let t = if state.enemies.get(target_slot)
        .and_then(|s| s.as_ref()).map_or(true, |e| e.hp <= 0.0)
    {
        match first_alive(state) { Some(i) => i, None => return }
    } else { target_slot };

    let eidolon = state.team[idx].eidolon;
    let member  = fua_member_clone(state, idx);

    let base_mult = if eidolon >= 6 { 1.10 + 1.40 } else { 1.10 }; // E6: +140%
    let toughness = if eidolon >= 4 { 20.0 } else { 10.0 };         // E4: ×2 toughness

    let fua = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       base_mult,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: toughness,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let dmg = state.enemies[t].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &fua))
        .unwrap_or(0.0);
    if dmg > 0.0 {
        if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
        if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[t] = None;
        }
    }

    // Refresh Talent +60% DMG (managed via on_turn_start)
    set(state, idx, TDMG_REM, 2.0);
    // E4: refresh +8% SPD
    if eidolon >= 4 {
        set(state, idx, E4_SPD_REM, 2.0);
    }
    // Mark talent used this Feixiao-turn
    set(state, idx, TALENT_USED, 1.0);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Talent FUA: {:.0} DMG ({}%{}) | FA {:.0}",
        dmg, (base_mult * 100.0) as i32,
        if eidolon >= 6 { "+E6" } else { "" },
        get(state, idx, FA),
    ));
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    // Feixiao uses FA (not energy) — set max_energy very high to skip engine ult check
    state.team[idx].max_energy = 1e13;
    state.team[idx].is_fua     = true;

    // Minor traces
    state.team[idx].buffs.atk_percent += 28.0;
    state.team[idx].buffs.crit_rate   += 12.0;
    state.team[idx].buffs.def_percent += 12.5;

    // A4: FUA CRIT DMG +36% (passive, managed per-FUA via clone; also +36% on ult)
    // Permanently applied here for A4's "Follow-up attacks' CRIT DMG +36%" on non-ult FUAs.
    // For ult hits we add it manually in on_ult.
    // (See approximation note: slightly over-estimates Basic/Skill CRIT DMG)

    // Initialise stacks
    for key in [FA, FA_ACC, TALENT_USED, E2_FUA_CT,
                TDMG_REM, TDMG_APPLD, A6_REM, A6_APPLD,
                E4_SPD_REM, E4_SPD_APPLD] {
        state.team[idx].stacks.insert(key, 0.0);
    }

    // A2: start with 3 FA
    set(state, idx, FA, 3.0);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.remove("_action_advance_pct");

    // A2: if no Talent FUA was launched last turn, count as +1 attack toward FA
    if get(state, idx, TALENT_USED) < 1.0 {
        let acc = get(state, idx, FA_ACC) + 0.5;
        if acc >= 1.0 {
            add_fa(state, idx, 1.0);
            set(state, idx, FA_ACC, acc - 1.0);
        } else {
            set(state, idx, FA_ACC, acc);
        }
    }

    // Reset per-turn counters
    set(state, idx, TALENT_USED,  0.0);
    set(state, idx, E2_FUA_CT,    0.0);

    // ── Timed buffs ─────────────────────────────────────────────────────────

    // Talent +60% DMG
    let (apply_tdmg, remove_tdmg) = tick_timed(state, idx, TDMG_REM, TDMG_APPLD);
    if apply_tdmg  { state.team[idx].buffs.dmg_boost   += 60.0; }
    if remove_tdmg { state.team[idx].buffs.dmg_boost   -= 60.0; }

    // A6 +48% ATK
    let (apply_a6, remove_a6) = tick_timed(state, idx, A6_REM, A6_APPLD);
    if apply_a6  { state.team[idx].buffs.atk_percent += 48.0; }
    if remove_a6 { state.team[idx].buffs.atk_percent -= 48.0; }

    // E4 +8% SPD
    let (apply_e4, remove_e4) = tick_timed(state, idx, E4_SPD_REM, E4_SPD_APPLD);
    if apply_e4  { state.team[idx].buffs.speed_percent += 8.0; }
    if remove_e4 { state.team[idx].buffs.speed_percent -= 8.0; }

    // Set ult readiness flag if FA ≥ 6
    let fa = get(state, idx, FA);
    if fa >= 6.0 {
        state.team[idx].stacks.insert("_ult_ready", 1.0);
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    // A6: +48% ATK when using Skill (buff tracked in stacks, kicks in next on_turn_start)
    // Nothing to do here for the action itself since the buff is stacks-managed.
    // (Skill won't benefit this turn; documented approximation)
    let _ = (state, idx, action); // suppress unused warnings
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Skill => {
            // A6: +48% ATK for 3 turns starting next turn
            set(state, idx, A6_REM, 3.0);

            // Skill directly triggers 1 Talent FUA (ignores TALENT_USED — not a Talent proc)
            let t = target_idx.or_else(|| first_alive(state));
            if let Some(slot) = t {
                fire_talent_fua(state, idx, slot);
                // Clear TALENT_USED after Skill-triggered FUA so an ally later this cycle
                // can still trigger the Talent (the Skill FUA is extra, not the Talent trigger)
                set(state, idx, TALENT_USED, 0.0);
            }
        }
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);

    // Consume 6 FA
    let fa = get(state, idx, FA);
    set(state, idx, FA, (fa - 6.0).max(0.0));

    let eidolon = state.team[idx].eidolon;

    if first_alive(state).is_none() { return; }

    // Sub-hits: 6 × 90% Wind ATK
    // Boltsunder Blitz (60% + 30% if broken) or Waraxe Skyward (60% + 30% if not broken)
    // One always gets +30%, so all sub-hits are effectively 90% ATK.
    let mut e1_stacks = 0.0f64;
    let mut total_dmg = 0.0f64;
    let name = state.team[idx].name.clone();

    for _ in 0..6 {
        let slot = match first_alive(state) { Some(i) => i, None => break };

        let mut m = state.team[idx].clone();
        // A4: ult = FUA → +36% CRIT DMG, include follow_up_dmg_boost
        m.buffs.crit_dmg  += 36.0;
        let fub = m.buffs.follow_up_dmg_boost;
        m.buffs.dmg_boost += fub;
        // E1: stacked ult DMG bonus from previous sub-hits (applied to ult_dmg_boost)
        m.buffs.ult_dmg_boost += e1_stacks * 10.0;
        // E6: +20% All-Type RES PEN on ult DMG
        if eidolon >= 6 { m.buffs.res_pen += 20.0; }

        let sub = ActionParams {
            action_type:      ActionType::Ultimate,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       0.90,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 10.0,
            inflicts_debuff:  false,
            is_ult_dmg:       true,
        };

        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&m, e, &sub))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            state.total_damage += dmg;
            total_dmg += dmg;
            if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[slot] = None;
            }
        }

        // E1: each sub-hit generates a stack (max 5)
        if eidolon >= 1 && e1_stacks < 5.0 {
            e1_stacks += 1.0;
        }
    }

    // Finisher: 160% ATK Wind (same A4/E1/E6 bonuses at max E1 stacks)
    if let Some(slot) = first_alive(state) {
        let mut m = state.team[idx].clone();
        m.buffs.crit_dmg  += 36.0;
        let fub = m.buffs.follow_up_dmg_boost;
        m.buffs.dmg_boost += fub;
        m.buffs.ult_dmg_boost += e1_stacks * 10.0; // max 5 stacks → +50%
        if eidolon >= 6 { m.buffs.res_pen += 20.0; }

        let fin = ActionParams {
            action_type:      ActionType::Ultimate,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       1.60,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 30.0,
            inflicts_debuff:  false,
            is_ult_dmg:       true,
        };

        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&m, e, &fin))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            state.total_damage += dmg;
            total_dmg += dmg;
            if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[slot] = None;
            }
        }
        state.add_log(&name, format!(
            "Ult (FA {:.0}→{:.0}): 6 sub-hits + finisher = {:.0} total DMG{}",
            fa, (fa - 6.0).max(0.0), total_dmg,
            if eidolon >= 1 { " (E1 stacks)" } else { "" },
        ));
    }
}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,
    _source_idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    let eidolon = state.team[idx].eidolon;

    // ── FA accumulation ──────────────────────────────────────────────────────
    // Feixiao's own ult attacks are dispatched with source_idx = feixiao_idx,
    // but on_ally_action skips source_idx == i, so we never count them. ✓
    let is_fua = action.action_type == ActionType::FollowUp;

    if is_fua && eidolon >= 2 {
        // E2: each ally FUA grants 1 FA directly (max 6 per Feixiao-turn)
        let ct = get(state, idx, E2_FUA_CT);
        if ct < 6.0 {
            add_fa(state, idx, 1.0);
            set(state, idx, E2_FUA_CT, ct + 1.0);
        }
    } else {
        // Base: 2 attacks = 1 FA (accumulator)
        let acc = get(state, idx, FA_ACC) + 0.5;
        if acc >= 1.0 {
            add_fa(state, idx, 1.0);
            set(state, idx, FA_ACC, acc - 1.0);
        } else {
            set(state, idx, FA_ACC, acc);
        }
    }

    // ── Talent FUA (once per Feixiao-turn) ───────────────────────────────────
    let talent_used = get(state, idx, TALENT_USED);
    if talent_used >= 1.0 { return; }

    let t = match target_idx.or_else(|| first_alive(state)) { Some(i) => i, None => return };
    fire_talent_fua(state, idx, t);
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

#[allow(dead_code)]
pub fn on_break(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
