use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ─── Stack keys ───────────────────────────────────────────────────────────────
const IN_CC:     &str = "ff_in_cc";       // 1 when Complete Combustion is active
const CC_TURNS:  &str = "ff_cc_turns";    // fractional enhanced turns remaining
const CC_BREAKS: &str = "ff_cc_breaks";   // A2: number of break-delays used this CC (max 3)
const E2_EXTRA:  &str = "ff_e2_extra";    // E2: extra turn already used this turn (0/1)

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn get(state: &SimState, idx: usize, key: &str) -> f64 {
    state.team[idx].stacks.get(key).copied().unwrap_or(0.0)
}

fn set(state: &mut SimState, idx: usize, key: &'static str, v: f64) {
    state.team[idx].stacks.insert(key, v);
}

fn in_cc(state: &SimState, idx: usize) -> bool {
    get(state, idx, IN_CC) >= 1.0
}

fn first_alive(state: &SimState) -> Option<usize> {
    state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
}

/// Total Break Effect % (base_stats + buffs.break_effect).
fn total_be(state: &SimState, idx: usize) -> f64 {
    state.team[idx].base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0)
        + state.team[idx].buffs.break_effect
}

/// Toughness dealt by an action, accounting for break efficiency.
fn toughness_dealt(break_eff: f64, base: f64) -> f64 {
    base * (1.0 + break_eff / 100.0)
}

/// Apply super break DMG from Firefly's enhanced action against a broken target.
fn fire_super_break(state: &mut SimState, idx: usize, t: usize, tgh_dealt: f64, super_mult: f64) {
    let dmg = {
        let member = &state.team[idx];
        state.enemies[t].as_ref()
            .map(|e| damage::calculate_super_break_damage(member, e, tgh_dealt, super_mult))
            .unwrap_or(0.0)
    };
    if dmg > 0.0 {
        if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
        if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[t] = None;
        }
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Super Break: {:.0} DMG (mult {:.0}%)", dmg, super_mult * 100.0));
    }
}

/// Enter Complete Combustion — apply all persistent CC bonuses.
fn enter_cc(state: &mut SimState, idx: usize) {
    // +60 flat SPD: add to base_stats directly (not snapshotted)
    *state.team[idx].base_stats.entry(ids::CHAR_SPD_ID.to_string()).or_insert(100.0) += 60.0;
    // A2: +25% Break Effect
    state.team[idx].buffs.break_effect += 25.0;
    let eidolon = state.team[idx].eidolon;
    // E4: +50% Effect RES in CC
    if eidolon >= 4 { state.team[idx].buffs.effect_res += 50.0; }
    // E6: +20% Fire RES PEN in CC
    if eidolon >= 6 { state.team[idx].buffs.res_pen    += 20.0; }
    // Talent: max DMG reduction (40%) during CC
    state.team[idx].buffs.incoming_dmg_reduction += 40.0;
    set(state, idx, IN_CC, 1.0);
}

/// Exit Complete Combustion — revert all persistent CC bonuses.
fn exit_cc(state: &mut SimState, idx: usize) {
    *state.team[idx].base_stats.entry(ids::CHAR_SPD_ID.to_string()).or_insert(160.0) -= 60.0;
    state.team[idx].buffs.break_effect         -= 25.0;
    let eidolon = state.team[idx].eidolon;
    if eidolon >= 4 { state.team[idx].buffs.effect_res -= 50.0; }
    if eidolon >= 6 { state.team[idx].buffs.res_pen    -= 20.0; }
    state.team[idx].buffs.incoming_dmg_reduction -= 40.0;
    set(state, idx, IN_CC,    0.0);
    set(state, idx, CC_TURNS, 0.0);
    set(state, idx, CC_BREAKS, 0.0);
    let name = state.team[idx].name.clone();
    state.add_log(&name, "Complete Combustion ended".to_string());
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 240.0;

    // Talent: if energy < 50% of max (< 120), regenerate to 50%
    if state.team[idx].energy < 120.0 {
        state.team[idx].energy = 120.0;
    }

    // Minor traces
    state.team[idx].buffs.break_effect += 37.3;
    *state.team[idx].base_stats.entry(ids::CHAR_SPD_ID.to_string()).or_insert(100.0) += 5.0;
    state.team[idx].buffs.effect_res   += 18.0;

    // A6: +0.8% BE per 10 ATK exceeding 1800
    let atk_base = state.team[idx].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0)
        + state.team[idx].lightcone.base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
    let atk_total = atk_base * (1.0 + state.team[idx].buffs.atk_percent / 100.0);
    if atk_total > 1800.0 {
        let excess_tens = ((atk_total - 1800.0) / 10.0).floor();
        state.team[idx].buffs.break_effect += excess_tens * 0.8;
    }

    // Initialise stacks
    for key in [IN_CC, CC_TURNS, CC_BREAKS, E2_EXTRA] {
        state.team[idx].stacks.insert(key, 0.0);
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.remove("_action_advance_pct");
    // Reset per-turn E2 flag
    set(state, idx, E2_EXTRA, 0.0);

    if !in_cc(state, idx) { return; }

    let cc_turns = get(state, idx, CC_TURNS);
    let new_turns = cc_turns - 1.0;
    if new_turns <= 0.0 {
        exit_cc(state, idx);
    } else {
        set(state, idx, CC_TURNS, new_turns);
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    if !in_cc(state, idx) { return; }

    let eidolon = state.team[idx].eidolon;
    let be = total_be(state, idx).min(360.0); // capped at 360% for scaling

    match action.action_type {
        ActionType::Basic => {
            // Enhanced Basic ATK: 150% ATK, 15 toughness
            action.multiplier      = 1.50;
            action.toughness_damage = 15.0;
        }
        ActionType::Skill => {
            // Enhanced Skill: (0.2×BE + 200%) ATK, 30 toughness
            action.multiplier      = (200.0 + 0.2 * be) / 100.0;
            action.toughness_damage = 30.0;
            // E1: ignore 15% DEF
            if eidolon >= 1 {
                state.team[idx].buffs.def_ignore += 15.0;
            }
        }
        _ => {}
    }

    // Ult +50% Break Efficiency during CC enhanced actions; E6 adds another +50%
    let eff_bonus = if eidolon >= 6 { 100.0 } else { 50.0 };
    state.team[idx].buffs.break_efficiency += eff_bonus;
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    let eidolon   = state.team[idx].eidolon;
    let cc_active = in_cc(state, idx);
    let t_slot    = target_idx.or_else(|| first_alive(state));

    // ── Revert on_before_action temporary boosts ──────────────────────────────
    if cc_active {
        let eff_bonus = if eidolon >= 6 { 100.0 } else { 50.0 };
        state.team[idx].buffs.break_efficiency -= eff_bonus;
        if eidolon >= 1 && action.action_type == ActionType::Skill {
            state.team[idx].buffs.def_ignore -= 15.0;
        }
    }

    // ── Energy correction ─────────────────────────────────────────────────────
    // Simulator auto-adds 20 on Basic and 30 on Skill. Override:
    match action.action_type {
        ActionType::Basic if cc_active => {
            // Enhanced Basic: 0 energy gain → undo simulator's +20
            state.team[idx].energy -= 20.0 * (1.0 + state.team[idx].buffs.energy_regen_rate / 100.0);
        }
        ActionType::Skill if !cc_active => {
            // Normal Skill: 60% of 240 = 144 energy → add 114 on top of simulator's +30
            let err = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
            state.team[idx].energy += 114.0 * err;
            // HP cost: 40% max HP (can't go below 1)
            let cost = state.team[idx].max_hp * 0.40;
            state.team[idx].hp = (state.team[idx].hp - cost).max(1.0);
            // Action advance +25%
            set(state, idx, "_action_advance_pct", 25.0);
        }
        ActionType::Skill if cc_active => {
            // Enhanced Skill: 0 energy gain → undo simulator's +30
            let err = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
            state.team[idx].energy -= 30.0 * err;
            // E1: no SP cost — refund SP consumed by simulator
            if eidolon >= 1 {
                state.skill_points = (state.skill_points + 1).min(5);
            }
        }
        _ => {}
    }

    // ── HP regen from Enhanced actions ────────────────────────────────────────
    if cc_active {
        let regen = match action.action_type {
            ActionType::Basic => state.team[idx].max_hp * 0.20,
            ActionType::Skill => state.team[idx].max_hp * 0.25,
            _ => 0.0,
        };
        if regen > 0.0 {
            let max_hp = state.team[idx].max_hp;
            state.team[idx].hp = (state.team[idx].hp + regen).min(max_hp);
        }
    }

    // ── Fire Weakness from Enhanced Skill ─────────────────────────────────────
    if cc_active && action.action_type == ActionType::Skill {
        if let Some(t) = t_slot {
            if let Some(e) = state.enemies[t].as_mut() {
                if !e.weaknesses.contains(&"Fire".to_string()) {
                    e.weaknesses.push("Fire".to_string());
                }
            }
        }
    }

    // ── A4: Super Break DMG when target is already broken ─────────────────────
    if cc_active {
        let be = total_be(state, idx);
        let super_mult = if be >= 300.0 { 1.5 } else if be >= 150.0 { 1.0 } else { 0.0 };

        if super_mult > 0.0 {
            if let Some(t) = t_slot {
                let already_broken = state.enemies[t].as_ref().map_or(false, |e| e.is_broken);
                if already_broken {
                    let base_tgh = action.toughness_damage;
                    let eff      = state.team[idx].buffs.break_efficiency;
                    let tgh      = toughness_dealt(eff, base_tgh);
                    if tgh > 0.0 {
                        fire_super_break(state, idx, t, tgh, super_mult);
                    }
                }
            }
        }
    }

    // ── Clamp energy to [0, max] ──────────────────────────────────────────────
    let max_e = state.team[idx].max_energy;
    state.team[idx].energy = state.team[idx].energy.clamp(0.0, max_e);
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);

    // Guard: can't use ult while already in CC
    if in_cc(state, idx) { return; }

    // Calculate CC duration in Firefly turns (accounts for 100% action advance giving +1 free turn)
    let ff_spd_base = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0)
        * (1.0 + state.team[idx].buffs.speed_percent / 100.0);
    let ff_spd_cc   = ff_spd_base + 60.0;
    let cc_timer_av = 10000.0 / 70.0; // countdown timer SPD = 70
    let cc_turns    = 1.0 + (cc_timer_av * ff_spd_cc / 10000.0).floor();

    enter_cc(state, idx);
    set(state, idx, CC_TURNS, cc_turns);

    // Ult gives back 5 energy; energy was zeroed by simulator before on_ult
    state.team[idx].energy = 5.0;

    // 100% action advance → set to 99% (engine clips at < 100) for near-zero next AV
    set(state, idx, "_action_advance_pct", 99.0);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Complete Combustion entered — {:.1} enhanced turns (SPD {:.0}→{:.0})",
        cc_turns, ff_spd_base, ff_spd_cc,
    ));
}

pub fn on_break(state: &mut SimState, idx: usize, _enemy_slot: usize) {
    // A2: delay countdown by 10% per break (up to 3 times) → +0.3 CC turns
    if !in_cc(state, idx) { return; }
    let breaks = get(state, idx, CC_BREAKS);
    if breaks >= 3.0 { return; }
    set(state, idx, CC_BREAKS, breaks + 1.0);
    let turns = get(state, idx, CC_TURNS);
    set(state, idx, CC_TURNS, turns + 0.3);
    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("A2 break delay: CC extended (+0.3 turns, {:.1} remain)", turns + 0.3));
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
