use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ─── Stack keys (static str → TeamMember.stacks) ─────────────────────────────
const ENHANCE:     &str = "dhil_enhance";      // 0-3 enhancement level
const OUTROAR:     &str = "dhil_outroar";      // carried Outroar stacks (E4)
const OUTROAR_REM: &str = "dhil_outroar_rem";  // turns before E4 carry expires
const E6_STACKS:   &str = "dhil_e6";           // E6 Imaginary RES PEN stacks (0-3)
const FIRE_NOW:    &str = "dhil_fire_now";     // flag: fire enhanced Basic this action

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Average Righteous Heart DMG% boost over N hits (stacks gained after each hit).
/// E1: 2 stacks/hit, max 10. Base: 1 stack/hit, max 6.
fn avg_rh_dmg_pct(hits: usize, eidolon: i32) -> f64 {
    let per_hit:    usize = if eidolon >= 1 { 2 } else { 1 };
    let max_stacks: usize = if eidolon >= 1 { 10 } else { 6 };
    let sum: f64 = (0..hits)
        .map(|i| (i * per_hit).min(max_stacks) as f64)
        .sum();
    sum / hits as f64 * 10.0   // % DMG boost
}

/// Average Outroar CRIT DMG% boost across all hits.
/// Outroar stacks are gained before each hit from `blast_start` onward (1-indexed).
/// Carry stacks from E4 are active from hit 1. All hits see the current stack count.
fn avg_outroar_crit_pct(hits: usize, blast_start: usize, carry: f64) -> f64 {
    let mut stacks = carry;
    let mut total  = 0.0;
    for i in 1..=hits {
        if i >= blast_start {
            stacks = (stacks + 1.0).min(4.0);
        }
        total += stacks;
    }
    total / hits as f64 * 12.0  // % CRIT DMG boost
}

/// Outroar stack count at end of the action (for E4 carry).
fn final_outroar(hits: usize, blast_start: usize, carry: f64) -> f64 {
    let mut stacks = carry;
    for i in 1..=hits {
        if i >= blast_start {
            stacks = (stacks + 1.0).min(4.0);
        }
    }
    stacks
}

fn adj_slots(target: usize, enemies: &[Option<crate::models::SimEnemy>]) -> Vec<usize> {
    let mut v = Vec::new();
    if target > 0 && enemies[target - 1].is_some() { v.push(target - 1); }
    if target + 1 < enemies.len() && enemies[target + 1].is_some() { v.push(target + 1); }
    v
}

// ─── Enhanced Basic ATK dispatcher ───────────────────────────────────────────

fn fire_enhanced_basic(
    state:      &mut SimState,
    idx:        usize,
    target_slot: usize,
    was_skill:  bool,
) {
    let enhance = state.team[idx].stacks.get(ENHANCE).copied().unwrap_or(0.0) as usize;
    let eidolon = state.team[idx].eidolon;
    let carry   = state.team[idx].stacks.get(OUTROAR).copied().unwrap_or(0.0);

    // Per-level lookup tables
    const HITS:          [usize; 4] = [2, 3, 5, 7];
    const MAIN_MULT:     [f64;   4] = [1.00, 2.60, 3.80, 5.00];
    const BLAST_MULT:    [f64;   4] = [0.0, 0.0, 1.20, 7.20]; // total per adj (2×60 / 4×180)
    const TOUGHNESS_M:   [f64;   4] = [10.0, 20.0, 30.0, 40.0];
    const TOUGHNESS_A:   [f64;   4] = [0.0, 0.0, 10.0, 20.0];
    const ENERGY_GAIN:   [f64;   4] = [20.0, 30.0, 35.0, 40.0];
    const BLAST_START:   [usize; 4] = [usize::MAX, usize::MAX, 4, 4];

    let n          = HITS[enhance];
    let rh_boost   = avg_rh_dmg_pct(n, eidolon);
    let or_boost   = avg_outroar_crit_pct(n, BLAST_START[enhance], carry);
    let final_or   = final_outroar(n, BLAST_START[enhance], carry);

    // A6: +24% CRIT DMG vs Imaginary-weak target
    let has_imag_weak = state.enemies[target_slot].as_ref()
        .map_or(false, |e| e.weaknesses.contains(&"Imaginary".to_string()));

    let mut member = state.team[idx].clone();
    member.buffs.dmg_boost += rh_boost;
    member.buffs.crit_dmg  += or_boost;
    if has_imag_weak { member.buffs.crit_dmg += 24.0; }

    // E6: consume RES PEN stacks on Fulgurant Leap
    if eidolon >= 6 && enhance == 3 {
        let e6 = state.team[idx].stacks.get(E6_STACKS).copied().unwrap_or(0.0);
        member.buffs.res_pen += e6 * 20.0;
        state.team[idx].stacks.remove(E6_STACKS);
    }

    // Main target
    let main_action = ActionParams {
        action_type:      ActionType::Basic,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       MAIN_MULT[enhance],
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: TOUGHNESS_M[enhance],
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let mut total_dmg = 0.0;
    let main_dmg = state.enemies[target_slot].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &main_action))
        .unwrap_or(0.0);
    if main_dmg > 0.0 {
        if let Some(e) = state.enemies[target_slot].as_mut() { e.hp -= main_dmg; }
        total_dmg += main_dmg;
    }
    if state.enemies[target_slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[target_slot] = None;
    }

    // Adjacent blast (Divine Spear / Fulgurant Leap)
    if BLAST_MULT[enhance] > 0.0 {
        let adjs = adj_slots(target_slot, &state.enemies);
        if !adjs.is_empty() {
            let blast_action = ActionParams {
                action_type:      ActionType::Basic,
                scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                multiplier:       BLAST_MULT[enhance],
                extra_multiplier: 0.0,
                extra_dmg:        0.0,
                toughness_damage: TOUGHNESS_A[enhance],
                inflicts_debuff:  false,
                is_ult_dmg:       false,
            };
            for &s in &adjs {
                let dmg = state.enemies[s].as_ref()
                    .map(|e| damage::calculate_damage(&member, e, &blast_action))
                    .unwrap_or(0.0);
                if dmg > 0.0 {
                    if let Some(e) = state.enemies[s].as_mut() { e.hp -= dmg; }
                    total_dmg += dmg;
                }
                if state.enemies[s].as_ref().map_or(false, |e| e.hp <= 0.0) {
                    state.enemies[s] = None;
                }
            }
        }
    }

    state.total_damage += total_dmg;

    // Update Outroar carry (E4: persists for one extra turn via rem counter)
    if eidolon >= 4 && enhance >= 2 && final_or > 0.0 {
        state.team[idx].stacks.insert(OUTROAR, final_or);
        state.team[idx].stacks.insert(OUTROAR_REM, 2.0); // survives next on_turn_start
    } else if eidolon < 4 || final_or <= 0.0 {
        state.team[idx].stacks.remove(OUTROAR);
        state.team[idx].stacks.remove(OUTROAR_REM);
    }
    // else (E4, enhance 0/1 with carry): leave OUTROAR + rem unchanged to tick naturally

    // Reset enhancement level
    state.team[idx].stacks.insert(ENHANCE, 0.0);

    // Energy correction: target = ENERGY_GAIN[enhance]; accounting gave 30 (Skill) or 20 (Basic)
    let err = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
    let given  = if was_skill { 30.0 } else { 20.0 } * err;
    let target = ENERGY_GAIN[enhance] * err;
    let max_e  = state.team[idx].max_energy;
    state.team[idx].energy = (state.team[idx].energy + target - given).clamp(0.0, max_e);

    let names = ["Beneficent Lotus", "Transcendence", "Divine Spear", "Fulgurant Leap"];
    let name  = state.team[idx].name.clone();
    state.add_log(&name, format!("{}: {:.0} DMG", names[enhance], total_dmg));
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 140.0;
    state.team[idx].buffs.dmg_boost  += 22.4; // Imaginary DMG minor trace
    state.team[idx].buffs.crit_rate  += 12.0; // CRIT Rate minor trace
    state.team[idx].buffs.hp_percent += 10.0; // HP minor trace

    state.team[idx].stacks.insert(ENHANCE, 0.0);

    // A2: +15 energy at battle start
    let err   = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
    let max_e = state.team[idx].max_energy;
    state.team[idx].energy = (state.team[idx].energy + 15.0 * err).min(max_e);

    // Technique: AoE 120% ATK to all enemies at battle start + gain 1 Squama
    state.stacks.insert("dhil_squama".to_string(), 1.0);

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if !alive.is_empty() {
        let member = state.team[idx].clone();
        let tech = ActionParams {
            action_type:      ActionType::Basic,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       1.20,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 0.0,
            inflicts_debuff:  false,
            is_ult_dmg:       false,
        };
        let mut total = 0.0;
        for &slot in &alive {
            let dmg = state.enemies[slot].as_ref()
                .map(|e| damage::calculate_damage(&member, e, &tech))
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
        state.add_log(&name, format!("Technique AoE: {:.0} DMG", total));
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // Clear E2 action advance (used for scheduling after previous ult)
    state.team[idx].stacks.remove("_action_advance_pct");

    // Tick E4 Outroar carry counter
    if state.team[idx].eidolon >= 4 {
        let rem = state.team[idx].stacks.get(OUTROAR_REM).copied().unwrap_or(0.0);
        if rem > 0.0 {
            if rem <= 1.0 {
                state.team[idx].stacks.remove(OUTROAR_REM);
                state.team[idx].stacks.remove(OUTROAR);
            } else {
                state.team[idx].stacks.insert(OUTROAR_REM, rem - 1.0);
            }
        }
    }
}

pub fn on_before_action(
    state:       &mut SimState,
    idx:         usize,
    action:      &mut ActionParams,
    _target_idx: Option<usize>,
) {
    let enhance = state.team[idx].stacks.get(ENHANCE).copied().unwrap_or(0.0) as i32;

    match action.action_type {
        ActionType::Skill => {
            if enhance < 3 {
                // Dracore Libre: free enhancement, no damage
                state.team[idx].stacks.insert(ENHANCE, (enhance + 1) as f64);
            } else {
                // enhance = 3: fire Fulgurant Leap on this Skill turn
                state.team[idx].stacks.insert(FIRE_NOW, 1.0);
            }
            action.multiplier       = 0.0;
            action.toughness_damage = 0.0;
        }
        ActionType::Basic => {
            // Fire at current enhancement level
            state.team[idx].stacks.insert(FIRE_NOW, 1.0);
            action.multiplier       = 0.0;
            action.toughness_damage = 0.0;
        }
        _ => {}
    }
}

pub fn on_after_action(
    state:      &mut SimState,
    idx:        usize,
    action:     &ActionParams,
    target_idx: Option<usize>,
) {
    let fire_now = state.team[idx].stacks.remove(FIRE_NOW).is_some();
    let is_skill = action.action_type == ActionType::Skill;

    if is_skill {
        // Dracore Libre never costs SP — refund the -1 the simulator took
        state.skill_points = (state.skill_points + 1).min(5);
        if !fire_now {
            // Enhancement-only turn: Dracore Libre gives 0 energy; undo the +30 from accounting
            let err   = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
            let max_e = state.team[idx].max_energy;
            state.team[idx].energy = (state.team[idx].energy - 30.0 * err).clamp(0.0, max_e);
        }
    }

    if fire_now {
        // Resolve target: prefer passed target, fall back to first living enemy
        let slot = target_idx
            .filter(|&t| state.enemies.get(t).and_then(|e| e.as_ref()).map_or(false, |e| e.hp > 0.0))
            .or_else(|| state.enemies.iter().position(|e| e.as_ref().map_or(false, |e| e.hp > 0.0)));

        if let Some(slot) = slot {
            fire_enhanced_basic(state, idx, slot, is_skill);
        }
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;

    // Find main target
    let target_slot = match state.enemies.iter()
        .position(|e| e.as_ref().map_or(false, |e| e.hp > 0.0))
    {
        Some(s) => s,
        None    => return,
    };

    // 3-hit ult → avg RH boost (3 hits), no Outroar
    let rh_boost = avg_rh_dmg_pct(3, eidolon);
    let has_imag_weak = state.enemies[target_slot].as_ref()
        .map_or(false, |e| e.weaknesses.contains(&"Imaginary".to_string()));

    let mut member = state.team[idx].clone();
    member.buffs.dmg_boost += rh_boost;
    if has_imag_weak { member.buffs.crit_dmg += 24.0; } // A6

    let main_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       3.00,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };
    let adj_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       1.40,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };

    let mut total_dmg = 0.0;

    // Main target
    let main_dmg = state.enemies[target_slot].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &main_action))
        .unwrap_or(0.0);
    if main_dmg > 0.0 {
        if let Some(e) = state.enemies[target_slot].as_mut() { e.hp -= main_dmg; }
        total_dmg += main_dmg;
    }
    if state.enemies[target_slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[target_slot] = None;
    }

    // Adjacent targets
    let adjs = adj_slots(target_slot, &state.enemies);
    for &s in &adjs {
        let dmg = state.enemies[s].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &adj_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[s].as_mut() { e.hp -= dmg; }
            total_dmg += dmg;
        }
        if state.enemies[s].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[s] = None;
        }
    }

    state.total_damage += total_dmg;

    // Gain 2 Squama (E2: +1 = 3)
    let gain    = if eidolon >= 2 { 3.0 } else { 2.0 };
    let current = state.stacks.get("dhil_squama").copied().unwrap_or(0.0);
    state.stacks.insert("dhil_squama".to_string(), (current + gain).min(3.0));

    // E2: advance action ~100%
    if eidolon >= 2 {
        state.team[idx].stacks.insert("_action_advance_pct", 99.9);
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Azure's Aqua: {:.0} DMG, +{:.0} Squama{}",
        total_dmg, gain,
        if eidolon >= 2 { ", advance" } else { "" }
    ));
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state:       &mut SimState,
    idx:         usize,
    _source_idx: usize,
    action:      &ActionParams,
    _target_idx: Option<usize>,
) {
    // E6: after any other ally uses Ultimate, gain +1 Imaginary RES PEN stack (max 3)
    // Stacks are consumed by Fulgurant Leap in fire_enhanced_basic
    if state.team[idx].eidolon >= 6 && action.action_type == ActionType::Ultimate {
        let cur = state.team[idx].stacks.get(E6_STACKS).copied().unwrap_or(0.0);
        if cur < 3.0 {
            state.team[idx].stacks.insert(E6_STACKS, cur + 1.0);
        }
    }
}
