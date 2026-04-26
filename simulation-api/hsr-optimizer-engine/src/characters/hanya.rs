use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ─── Global state.stacks keys ─────────────────────────────────────────────────
const BURDEN_SLOT: &str = "hanya_burden_slot";  // enemy slot (-1.0 = inactive)
const BURDEN_HITS: &str = "hanya_burden_hits";  // 0–1 hit count toward next SP
const BURDEN_SP:   &str = "hanya_burden_sp";    // SP recoveries so far (max 2)
const ULT_TARGET:  &str = "hanya_ult_target";   // buffed ally idx (-1.0 = inactive)
const ULT_SPD_INC: &str = "hanya_ult_spd_inc";  // flat SPD added to Ult target
const E1_USED:     &str = "hanya_e1_used";      // 1.0 once E1 advance fires this Hanya turn
const E2_SPD_INC:  &str = "hanya_e2_spd_inc";   // flat SPD added to Hanya from E2
const E2_REM:      &str = "hanya_e2_rem";        // E2 turns remaining

// ─── Per-ally String keys ─────────────────────────────────────────────────────
fn talent_key(i: usize) -> String { format!("hanya_talent_{}", i) } // turns remaining
fn a2_key(i: usize)     -> String { format!("hanya_a2_{}", i) }     // ATK buff turns
fn ult_key(i: usize)    -> String { format!("hanya_ult_{}", i) }    // Ult buff turns

// ─── Buff management helpers ──────────────────────────────────────────────────

/// Apply Talent +30% (or +40% E6) DMG buff to ally. Only adds to dmg_boost on first apply.
fn apply_talent_buff(state: &mut SimState, ally: usize, eidolon: i32) {
    let bonus = if eidolon >= 6 { 40.0 } else { 30.0 };
    let rem   = state.stacks.get(&talent_key(ally)).copied().unwrap_or(0.0);
    if rem <= 0.0 {
        state.team[ally].buffs.dmg_boost += bonus;
    }
    state.stacks.insert(talent_key(ally), 2.0);
}

/// Tick Talent buff for ally (decrement; remove and revert when it reaches 0).
fn tick_talent_buff(state: &mut SimState, ally: usize, eidolon: i32) {
    let rem = state.stacks.get(&talent_key(ally)).copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }
    let bonus = if eidolon >= 6 { 40.0 } else { 30.0 };
    if rem <= 1.0 {
        state.team[ally].buffs.dmg_boost -= bonus;
        state.stacks.remove(&talent_key(ally));
    } else {
        state.stacks.insert(talent_key(ally), rem - 1.0);
    }
}

/// Apply A2 +10% ATK buff to ally (only adds on first apply; duration 1 action).
fn apply_a2_buff(state: &mut SimState, ally: usize) {
    let rem = state.stacks.get(&a2_key(ally)).copied().unwrap_or(0.0);
    if rem <= 0.0 {
        state.team[ally].buffs.atk_percent += 10.0;
    }
    state.stacks.insert(a2_key(ally), 1.0);
}

/// Tick A2 buff for ally.
fn tick_a2_buff(state: &mut SimState, ally: usize) {
    let rem = state.stacks.get(&a2_key(ally)).copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }
    if rem <= 1.0 {
        state.team[ally].buffs.atk_percent -= 10.0;
        state.stacks.remove(&a2_key(ally));
    } else {
        state.stacks.insert(a2_key(ally), rem - 1.0);
    }
}

/// Tick Ult SPD/ATK buff for ally.
fn tick_ult_buff(state: &mut SimState, ally: usize) {
    let rem = state.stacks.get(&ult_key(ally)).copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }
    if rem <= 1.0 {
        state.team[ally].buffs.atk_percent -= 60.0;
        // Revert SPD boost if this is the stored Ult target
        let ult_t = state.stacks.get(ULT_TARGET).copied().unwrap_or(-1.0);
        if ult_t as usize == ally {
            let spd_inc = state.stacks.get(ULT_SPD_INC).copied().unwrap_or(0.0);
            if spd_inc > 0.0 {
                let cur = state.team[ally].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
                state.team[ally].base_stats.insert(ids::CHAR_SPD_ID.to_string(), cur - spd_inc);
                state.stacks.insert(ULT_SPD_INC.to_string(), 0.0);
            }
            state.stacks.insert(ULT_TARGET.to_string(), -1.0);
        }
        state.stacks.remove(&ult_key(ally));
    } else {
        state.stacks.insert(ult_key(ally), rem - 1.0);
    }
}

/// Apply Burden to enemy `slot` (overwrites any existing Burden).
fn apply_burden(state: &mut SimState, slot: usize) {
    state.stacks.insert(BURDEN_SLOT.to_string(), slot as f64);
    state.stacks.insert(BURDEN_HITS.to_string(), 0.0);
    state.stacks.insert(BURDEN_SP.to_string(), 0.0);
}

/// Process one qualifying hit on the Burden target by `source_idx`.
/// `hanya_idx` = Hanya's team slot (for energy/log).
fn process_burden_hit(state: &mut SimState, hanya_idx: usize, source_idx: usize) {
    let eidolon = state.team[hanya_idx].eidolon;
    let hits    = state.stacks.get(BURDEN_HITS).copied().unwrap_or(0.0) + 1.0;

    if hits >= 2.0 {
        // SP recovery
        state.skill_points = (state.skill_points + 1).min(5);
        state.stacks.insert(BURDEN_HITS.to_string(), 0.0);

        // A6: Hanya regenerates 2 Energy per SP recovery
        let max_e = state.team[hanya_idx].max_energy;
        state.team[hanya_idx].energy = (state.team[hanya_idx].energy + 2.0).min(max_e);

        // A2: triggering ally gets +10% ATK for 1 action
        apply_a2_buff(state, source_idx);

        let sp = state.stacks.get(BURDEN_SP).copied().unwrap_or(0.0) + 1.0;
        state.stacks.insert(BURDEN_SP.to_string(), sp);

        // Auto-dispel after 2 recoveries
        if sp >= 2.0 {
            state.stacks.insert(BURDEN_SLOT.to_string(), -1.0);
        }

        let name = state.team[hanya_idx].name.clone();
        state.add_log(&name, format!(
            "Burden: SP +1 (×{:.0}/2 SP, A6 +2 Energy){}",
            sp, if sp >= 2.0 { " — Burden dispelled" } else { "" },
        ));
    } else {
        state.stacks.insert(BURDEN_HITS.to_string(), hits);
    }

    // Talent: DMG buff on source for 2 turns
    apply_talent_buff(state, source_idx, eidolon);
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 140.0;

    // Minor traces: +28% ATK, +9 flat SPD, +10% HP
    state.team[idx].buffs.atk_percent += 28.0;
    state.team[idx].buffs.hp_percent  += 10.0;
    let cur_spd = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), cur_spd + 9.0);

    // Init global state
    state.stacks.insert(BURDEN_SLOT.to_string(), -1.0);
    state.stacks.insert(BURDEN_HITS.to_string(),  0.0);
    state.stacks.insert(BURDEN_SP.to_string(),    0.0);
    state.stacks.insert(ULT_TARGET.to_string(),  -1.0);
    state.stacks.insert(ULT_SPD_INC.to_string(),  0.0);
    state.stacks.insert(E1_USED.to_string(),       0.0);
    state.stacks.insert(E2_SPD_INC.to_string(),    0.0);
    state.stacks.insert(E2_REM.to_string(),        0.0);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // Reset E1 action-advance flag each of Hanya's own turns
    state.stacks.insert(E1_USED.to_string(), 0.0);
    state.team[idx].stacks.remove("_action_advance_pct");

    let eidolon = state.team[idx].eidolon;

    // Tick Hanya's own timed buffs
    tick_talent_buff(state, idx, eidolon);
    tick_a2_buff(state, idx);
    tick_ult_buff(state, idx);

    // Tick E2 SPD buff
    let e2_rem = state.stacks.get(E2_REM).copied().unwrap_or(0.0);
    if e2_rem > 0.0 {
        let new_rem = e2_rem - 1.0;
        if new_rem <= 0.0 {
            let spd_inc = state.stacks.get(E2_SPD_INC).copied().unwrap_or(0.0);
            if spd_inc > 0.0 {
                let cur = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
                state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), cur - spd_inc);
                state.stacks.insert(E2_SPD_INC.to_string(), 0.0);
            }
            state.stacks.insert(E2_REM.to_string(), 0.0);
        } else {
            state.stacks.insert(E2_REM.to_string(), new_rem);
        }
    }
}

pub fn on_before_action(
    _state: &mut SimState,
    _idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            action.multiplier       = 1.00;
            action.toughness_damage = 10.0;
        }
        ActionType::Skill => {
            action.multiplier       = 2.40;
            action.toughness_damage = 20.0;
        }
        ActionType::Ultimate => {
            // Ult buffs an ally — no damage
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
    let burden_slot = state.stacks.get(BURDEN_SLOT).copied().unwrap_or(-1.0);

    match action.action_type {
        ActionType::Basic => {
            let t = match target_idx.or_else(|| {
                state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
            }) {
                Some(t) => t,
                None    => return,
            };

            // Basic ATK on Burden target: process hit
            if burden_slot >= 0.0 && t == burden_slot as usize {
                let bs       = burden_slot as usize;
                let sp_before = state.stacks.get(BURDEN_SP).copied().unwrap_or(0.0);
                process_burden_hit(state, idx, idx);
                // A4: +1 SP if Burden enemy died and sp was ≤ 1 before this hit
                let dead = state.enemies.get(bs).map_or(true, |s| s.as_ref().map_or(true, |e| e.hp <= 0.0));
                if dead && sp_before <= 1.0 && state.stacks.get(BURDEN_SLOT).copied().unwrap_or(-1.0) >= 0.0 {
                    state.skill_points = (state.skill_points + 1).min(5);
                    state.stacks.insert(BURDEN_SLOT.to_string(), -1.0);
                    let name = state.team[idx].name.clone();
                    state.add_log(&name, "A4: Burden enemy defeated (early) — +1 SP".to_string());
                }
            }
        }

        ActionType::Skill => {
            let t = match target_idx.or_else(|| {
                state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
            }) {
                Some(t) => t,
                None    => return,
            };
            // Apply Burden to the Skill target (moves from any previous target)
            apply_burden(state, t);
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!("Skill: Burden applied to enemy {}", t));

            // E2: +20% SPD for 1 of Hanya's turns
            let eidolon = state.team[idx].eidolon;
            if eidolon >= 2 {
                let base_spd = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
                let spd_inc  = base_spd * 0.20;
                state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), base_spd + spd_inc);
                state.stacks.insert(E2_SPD_INC.to_string(), spd_inc);
                state.stacks.insert(E2_REM.to_string(), 1.0);
            }
        }

        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;
    let ult_dur = if eidolon >= 4 { 3.0 } else { 2.0 }; // E4: +1 turn

    // Remove old Ult buff if still active (re-application)
    let old_t = state.stacks.get(ULT_TARGET).copied().unwrap_or(-1.0);
    if old_t >= 0.0 {
        let ot = old_t as usize;
        if state.stacks.get(&ult_key(ot)).copied().unwrap_or(0.0) > 0.0 {
            state.team[ot].buffs.atk_percent -= 60.0;
            let spd_inc = state.stacks.get(ULT_SPD_INC).copied().unwrap_or(0.0);
            if spd_inc > 0.0 {
                let cur = state.team[ot].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
                state.team[ot].base_stats.insert(ids::CHAR_SPD_ID.to_string(), cur - spd_inc);
                state.stacks.insert(ULT_SPD_INC.to_string(), 0.0);
            }
            state.stacks.remove(&ult_key(ot));
        }
        state.stacks.insert(ULT_TARGET.to_string(), -1.0);
    }

    // Target: highest-ATK non-Hanya ally
    let target = (0..state.team.len())
        .filter(|&i| i != idx && !state.team[i].is_downed)
        .max_by(|&a, &b| {
            let atk_a = state.team[a].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
            let atk_b = state.team[b].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
            atk_a.partial_cmp(&atk_b).unwrap_or(std::cmp::Ordering::Equal)
        });

    if let Some(t) = target {
        // SPD boost: +20% of Hanya's current total SPD (flat increment to ally base_stats)
        let hanya_base = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        let hanya_spd  = hanya_base * (1.0 + state.team[idx].buffs.speed_percent / 100.0);
        let spd_inc    = hanya_spd * 0.20;

        let ally_spd = state.team[t].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        state.team[t].base_stats.insert(ids::CHAR_SPD_ID.to_string(), ally_spd + spd_inc);

        // ATK boost: +60%
        state.team[t].buffs.atk_percent += 60.0;

        state.stacks.insert(ULT_TARGET.to_string(), t as f64);
        state.stacks.insert(ULT_SPD_INC.to_string(), spd_inc);
        state.stacks.insert(ult_key(t), ult_dur);

        let name   = state.team[idx].name.clone();
        let t_name = state.team[t].name.clone();
        state.add_log(&name, format!(
            "Ult: {} +{:.1} SPD, +60% ATK ({:.0}t{})",
            t_name, spd_inc, ult_dur,
            if eidolon >= 4 { ", E4+1t" } else { "" },
        ));
    }
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,
    source_idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    let eidolon = state.team[idx].eidolon;

    // Tick all timed buffs for the ally that just acted
    tick_talent_buff(state, source_idx, eidolon);
    tick_a2_buff(state, source_idx);
    tick_ult_buff(state, source_idx);

    // Check if this action hit the Burden target
    let burden_slot = state.stacks.get(BURDEN_SLOT).copied().unwrap_or(-1.0);
    if burden_slot >= 0.0
        && matches!(action.action_type, ActionType::Basic | ActionType::Skill | ActionType::Ultimate)
    {
        let bs = burden_slot as usize;
        let burden_alive = state.enemies.get(bs).map_or(false, |s| s.as_ref().map_or(false, |e| e.hp > 0.0));

        let hits = match target_idx {
            Some(t) => t == bs && burden_alive,
            None    => burden_alive, // AoE: assume hits Burden target
        };

        if hits {
            let sp_before = state.stacks.get(BURDEN_SP).copied().unwrap_or(0.0);
            process_burden_hit(state, idx, source_idx);

            // A4: +1 SP if Burden enemy died and sp count was ≤ 1 before this hit
            let dead = state.enemies.get(bs).map_or(true, |s| s.as_ref().map_or(true, |e| e.hp <= 0.0));
            if dead && sp_before <= 1.0 && state.stacks.get(BURDEN_SLOT).copied().unwrap_or(-1.0) >= 0.0 {
                state.skill_points = (state.skill_points + 1).min(5);
                state.stacks.insert(BURDEN_SLOT.to_string(), -1.0);
                let name = state.team[idx].name.clone();
                state.add_log(&name, "A4: Burden enemy defeated (early) — +1 SP".to_string());
            }

            // E1: advance Hanya's action 15% when Ult-buffed ally defeats an enemy
            if eidolon >= 1 {
                let ult_t    = state.stacks.get(ULT_TARGET).copied().unwrap_or(-1.0);
                let e1_used  = state.stacks.get(E1_USED).copied().unwrap_or(0.0);
                let enemy_dead = state.enemies.get(bs).map_or(true, |s| s.as_ref().map_or(true, |e| e.hp <= 0.0));

                if enemy_dead && ult_t as usize == source_idx && e1_used < 1.0 {
                    state.team[idx].stacks.insert("_action_advance_pct", 15.0);
                    state.stacks.insert(E1_USED.to_string(), 1.0);
                    let name = state.team[idx].name.clone();
                    state.add_log(&name, "E1: action advance 15% (Ult ally kill)".to_string());
                }
            }
        }
    }

    // Also E1 for any enemy defeat by Ult ally regardless of Burden
    if eidolon >= 1 {
        let ult_t   = state.stacks.get(ULT_TARGET).copied().unwrap_or(-1.0);
        let e1_used = state.stacks.get(E1_USED).copied().unwrap_or(0.0);
        if ult_t as usize == source_idx && e1_used < 1.0 {
            if let Some(t) = target_idx {
                let target_dead = state.enemies.get(t).map_or(false, |s| s.is_none());
                if target_dead {
                    state.team[idx].stacks.insert("_action_advance_pct", 15.0);
                    state.stacks.insert(E1_USED.to_string(), 1.0);
                    let name = state.team[idx].name.clone();
                    state.add_log(&name, "E1: action advance 15% (Ult ally kill)".to_string());
                }
            }
        }
    }
}
