use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimEnemy, SimState, StatusEffect, TeamMember};

// ─── Arcana cap ────────────────────────────────────────────────────────────────
fn arcana_cap(eidolon: i32) -> f64 {
    if eidolon >= 6 { 80.0 } else { 50.0 }
}

// ─── Per-enemy state (state.stacks, String keys) ──────────────────────────────
fn arcana_key(s: usize)     -> String { format!("bs_arcana_{}", s) }      // stack count
fn arcana_acc_key(s: usize) -> String { format!("bs_arcana_acc_{}", s) }  // shared accumulator
fn epiphany_key(s: usize)   -> String { format!("bs_epiphany_{}", s) }    // turns remaining

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn get_arcana(state: &SimState, slot: usize) -> f64 {
    state.stacks.get(&arcana_key(slot)).copied().unwrap_or(0.0)
}

fn is_epiphany(state: &SimState, slot: usize) -> bool {
    state.stacks.get(&epiphany_key(slot)).copied().unwrap_or(0.0) > 0.0
}

/// Add `n` Arcana stacks to `slot`.
/// `from_bs = true` → E6 doubles the infliction (BS inflicts).
/// Epiphany 50% chain → EV 2× applied when active.
fn add_arcana(state: &mut SimState, bs_idx: usize, slot: usize, n: f64, from_bs: bool) {
    if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { return; }
    let eidolon = state.team[bs_idx].eidolon;

    let mut effective = n;
    if from_bs && eidolon >= 6 { effective *= 2.0; } // E6: BS doubles all its Arcana inflictions
    if is_epiphany(state, slot)  { effective *= 2.0; } // Epiphany: 50% chain → EV 2×

    let cur = state.stacks.get(&arcana_key(slot)).copied().unwrap_or(0.0);
    state.stacks.insert(arcana_key(slot), cur + effective);

    // E1: -25% All RES while Arcana active (applied once; apply_enemy_debuff idempotent at same duration)
    if eidolon >= 1 {
        if let Some(e) = state.enemies[slot].as_mut() {
            effects::apply_enemy_debuff(e, "bs_e1_res", StatusEffect {
                duration: 999,
                value:    25.0,
                stat:     Some("All RES".to_string()),
                effects:  vec![],
            });
        }
    }
}

/// Remove all Arcana stacks and clean up E1 RES debuff.
fn clear_arcana(state: &mut SimState, slot: usize) {
    state.stacks.remove(&arcana_key(slot));
    if let Some(e) = state.enemies[slot].as_mut() {
        e.active_debuffs.remove("bs_e1_res");
        effects::recompute_enemy_caches(e);
    }
}

/// Apply DEF reduction from Skill/A4 (100% base chance — always lands).
fn apply_def_reduction(enemy: &mut SimEnemy) {
    effects::apply_enemy_debuff(enemy, "bs_skill_def", StatusEffect {
        duration: 3,
        value:    20.8,
        stat:     Some("DEF Reduction".to_string()),
        effects:  vec![],
    });
}

/// Apply or refresh Epiphany on `slot` (2 turns).
/// E4 adds +20% additional vulnerability (25% base → 45%).
fn apply_epiphany(state: &mut SimState, bs_idx: usize, slot: usize) {
    if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { return; }
    let eidolon = state.team[bs_idx].eidolon;
    let vuln    = if eidolon >= 4 { 45.0 } else { 25.0 };

    state.stacks.insert(epiphany_key(slot), 2.0);
    if let Some(e) = state.enemies[slot].as_mut() {
        effects::apply_enemy_buff(e, "bs_epiphany_vuln", StatusEffect {
            duration: 999, // manually managed via state.stacks
            value:    vuln,
            stat:     Some("Vulnerability".to_string()),
            effects:  vec![],
        });
    }
}

/// Arcana / adjacent DoT damage: no crit, +20% DEF ignore (Arcana passive).
fn calc_arcana_dmg(member: &TeamMember, target: &SimEnemy, mult: f64) -> f64 {
    let mut dot = member.clone();
    dot.buffs.crit_rate  = 0.0;   // DoTs never crit
    dot.buffs.def_ignore += 20.0; // Arcana ignores 20% DEF
    damage::calculate_damage(&dot, target, &ActionParams {
        action_type:      ActionType::TalentProc,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       mult,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 0.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    })
}

/// Probabilistic Arcana gain: add 0.65 to accumulator; when ≥ 1.0, inflict `n_per_proc` stacks.
fn try_add_arcana(state: &mut SimState, bs_idx: usize, slot: usize, n_per_proc: f64, from_bs: bool) {
    if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { return; }
    let acc = state.stacks.get(&arcana_acc_key(slot)).copied().unwrap_or(0.0) + 0.65;
    if acc >= 1.0 {
        add_arcana(state, bs_idx, slot, n_per_proc, from_bs);
        state.stacks.insert(arcana_acc_key(slot), acc - 1.0);
    } else {
        state.stacks.insert(arcana_acc_key(slot), acc);
    }
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 120.0;

    // Minor traces: +28% ATK, +14.4% Wind DMG, +10% EHR
    state.team[idx].buffs.atk_percent     += 28.0;
    state.team[idx].buffs.dmg_boost       += 14.4;
    state.team[idx].buffs.effect_hit_rate += 10.0;

    let n       = state.team.len();
    let eidolon = state.team[idx].eidolon;

    // A6: all allies gain DMG boost = 60% of BS's EHR (max 72%)
    let total_ehr = state.team[idx].base_stats.get(ids::CHAR_EHR_ID).copied().unwrap_or(0.0)
                  + state.team[idx].buffs.effect_hit_rate;
    let a6_boost = (total_ehr * 0.60).min(72.0);
    for i in 0..n {
        state.team[i].buffs.dmg_boost += a6_boost;
    }

    // Battle-start effects on all enemies
    let enemy_count = state.enemies.len();
    for slot in 0..enemy_count {
        if state.enemies[slot].is_none() { continue; }

        // A4: 100% DEF reduction (3 turns)
        if let Some(e) = state.enemies[slot].as_mut() { apply_def_reduction(e); }

        // E2: 30 Arcana; else A4: 65% × 1 Arcana
        if eidolon >= 2 {
            add_arcana(state, idx, slot, 30.0, true);
        } else {
            try_add_arcana(state, idx, slot, 1.0, true);
        }
    }
}

pub fn on_turn_start(_state: &mut SimState, _idx: usize) {}

pub fn on_before_action(
    _state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            action.multiplier       = 1.00;
            action.toughness_damage = 10.0;
            action.inflicts_debuff  = true;
        }
        ActionType::Skill => {
            // Main target: 90% ATK; adjacent computed manually in on_after_action
            action.multiplier       = 0.90;
            action.toughness_damage = 20.0;
            action.inflicts_debuff  = true;
        }
        _ => {}
    }
    let _ = (idx, target_idx);
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    let t = match target_idx.or_else(|| {
        state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
    }) {
        Some(t) => t,
        None    => return,
    };

    match action.action_type {
        ActionType::Basic => {
            // A4: DEF reduction on Basic ATK hit
            if let Some(e) = state.enemies[t].as_mut() { apply_def_reduction(e); }
            // A2: 65% × 5 Arcana
            try_add_arcana(state, idx, t, 5.0, true);
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!("A2 Basic: {:.0} Arcana on enemy {}", get_arcana(state, t), t));
        }

        ActionType::Skill => {
            // Adjacent enemies
            let adj: Vec<usize> = state.enemies.iter().enumerate()
                .filter(|(i, s)| *i != t && s.as_ref().map_or(false, |e| e.hp > 0.0))
                .map(|(i, _)| i)
                .take(2)
                .collect();

            for &a in &adj {
                let adj_action = ActionParams {
                    action_type:      ActionType::Skill,
                    scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                    multiplier:       0.90,
                    extra_multiplier: 0.0,
                    extra_dmg:        0.0,
                    toughness_damage: 10.0,
                    inflicts_debuff:  false,
                    is_ult_dmg:       false,
                };
                let dmg = {
                    let m = &state.team[idx];
                    state.enemies[a].as_ref()
                        .map(|e| damage::calculate_damage(m, e, &adj_action))
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
                if let Some(e) = state.enemies[a].as_mut() { apply_def_reduction(e); }
                try_add_arcana(state, idx, a, 5.0, true);
            }

            // Main target: DEF reduction + A2
            if let Some(e) = state.enemies[t].as_mut() { apply_def_reduction(e); }
            try_add_arcana(state, idx, t, 5.0, true);
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!("A2 Skill: {:.0} Arcana on enemy {}", get_arcana(state, t), t));
        }

        ActionType::Ultimate => {
            // A4: DEF reduction + A2 on all hit targets after Ult
            let enemy_count = state.enemies.len();
            for slot in 0..enemy_count {
                if let Some(e) = state.enemies[slot].as_mut() { apply_def_reduction(e); }
                try_add_arcana(state, idx, slot, 5.0, true);
            }
        }

        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    let eidolon     = state.team[idx].eidolon;
    let name        = state.team[idx].name.clone();
    let enemy_count = state.enemies.len();

    // Apply Epiphany to all enemies first (vulnerability active for this Ult's own DMG calc)
    for slot in 0..enemy_count {
        apply_epiphany(state, idx, slot);
    }

    // AoE 120% ATK Wind DMG
    let mut total_dmg = 0.0;
    for slot in 0..enemy_count {
        if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }
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
    }

    // E4: +8 energy when any enemy is in Epiphany state at Ult cast
    if eidolon >= 4 {
        let max_e = state.team[idx].max_energy;
        state.team[idx].energy = (state.team[idx].energy + 8.0).min(max_e);
    }

    state.add_log(&name, format!(
        "Ult: Epiphany (2t, +{}% DMG taken) applied + {:.0} Wind AoE DMG",
        if eidolon >= 4 { 45 } else { 25 }, total_dmg,
    ));
}

pub fn on_enemy_turn_start(state: &mut SimState, idx: usize, enemy_idx: usize) {
    if state.enemies[enemy_idx].as_ref().map_or(true, |e| e.hp <= 0.0) { return; }
    let eidolon = state.team[idx].eidolon;

    // ── Epiphany duration tick ─────────────────────────────────────────────────
    let epiphany_before = state.stacks.get(&epiphany_key(enemy_idx)).copied().unwrap_or(0.0);
    let had_epiphany    = epiphany_before > 0.0;

    if had_epiphany {
        let new_turns = epiphany_before - 1.0;
        if new_turns <= 0.0 {
            state.stacks.remove(&epiphany_key(enemy_idx));
            if let Some(e) = state.enemies[enemy_idx].as_mut() {
                e.active_buffs.remove("bs_epiphany_vuln");
                effects::recompute_enemy_caches(e);
            }
        } else {
            state.stacks.insert(epiphany_key(enemy_idx), new_turns);
        }
    }

    // E4: +8 energy at start of enemy turn while Epiphany was active this turn
    if eidolon >= 4 && had_epiphany {
        let max_e = state.team[idx].max_energy;
        state.team[idx].energy = (state.team[idx].energy + 8.0).min(max_e);
    }

    // ── Arcana DoT ────────────────────────────────────────────────────────────
    let arcana = get_arcana(state, enemy_idx);
    if arcana <= 0.0 { return; }

    let mult = 2.40 + 0.12 * arcana; // (240% + 12%/stack) of ATK

    // Main Arcana Wind DoT
    let dot_dmg = {
        let m = &state.team[idx];
        state.enemies[enemy_idx].as_ref()
            .map(|e| calc_arcana_dmg(m, e, mult))
            .unwrap_or(0.0)
    };
    if dot_dmg > 0.0 {
        if let Some(e) = state.enemies[enemy_idx].as_mut() { e.hp -= dot_dmg; }
        state.total_damage += dot_dmg;
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!(
            "Arcana DoT: {:.0} DMG (enemy {}, {:.0} stacks, {:.0}% ATK)",
            dot_dmg, enemy_idx, arcana, mult * 100.0,
        ));
        let dead = state.enemies[enemy_idx].as_ref().map_or(false, |e| e.hp <= 0.0);
        if dead {
            state.enemies[enemy_idx] = None;
            // E4: +8 energy when Epiphany enemy is defeated
            if eidolon >= 4 && had_epiphany {
                let max_e = state.team[idx].max_energy;
                state.team[idx].energy = (state.team[idx].energy + 8.0).min(max_e);
            }
        }
    }

    // Adjacent Wind DoT (180% ATK) — only when main target still alive
    if state.enemies[enemy_idx].as_ref().map_or(false, |e| e.hp > 0.0) {
        let adj: Vec<usize> = state.enemies.iter().enumerate()
            .filter(|(i, s)| *i != enemy_idx && s.as_ref().map_or(false, |e| e.hp > 0.0))
            .map(|(i, _)| i)
            .take(2)
            .collect();
        for &a in &adj {
            let adj_dmg = {
                let m = &state.team[idx];
                state.enemies[a].as_ref()
                    .map(|e| calc_arcana_dmg(m, e, 1.80))
                    .unwrap_or(0.0)
            };
            if adj_dmg > 0.0 {
                if let Some(e) = state.enemies[a].as_mut() { e.hp -= adj_dmg; }
                state.total_damage += adj_dmg;
                let name = state.team[idx].name.clone();
                state.add_log(&name, format!("Arcana adj DoT: {:.0} DMG (enemy {})", adj_dmg, a));
                let adj_had_epiphany = state.stacks.get(&epiphany_key(a)).copied().unwrap_or(0.0) > 0.0;
                let dead = state.enemies[a].as_ref().map_or(false, |e| e.hp <= 0.0);
                if dead {
                    state.enemies[a] = None;
                    if eidolon >= 4 && adj_had_epiphany {
                        let max_e = state.team[idx].max_energy;
                        state.team[idx].energy = (state.team[idx].energy + 8.0).min(max_e);
                    }
                }
            }
        }
    }

    // ── Talent: 65% Arcana per DoT instance (1 real DoT = Arcana itself) ──────
    try_add_arcana(state, idx, enemy_idx, 1.0, true);

    // ── Post-DoT: halve stacks (skip during Epiphany) and apply cap ───────────
    if state.enemies[enemy_idx].is_none() {
        // Enemy died from DoT — clean up any remaining stack state
        state.stacks.remove(&arcana_key(enemy_idx));
        state.stacks.remove(&arcana_acc_key(enemy_idx));
        return;
    }

    let cur         = get_arcana(state, enemy_idx);
    let epiphany_on = is_epiphany(state, enemy_idx);
    let cap         = arcana_cap(eidolon);

    if epiphany_on {
        // Epiphany: no halving, just cap
        state.stacks.insert(arcana_key(enemy_idx), cur.min(cap));
    } else {
        let halved = (cur / 2.0).floor().min(cap);
        if halved > 0.0 {
            state.stacks.insert(arcana_key(enemy_idx), halved);
        } else {
            clear_arcana(state, enemy_idx);
        }
    }
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,
    _source_idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // E6: 65% chance for 1 Arcana when a teammate attacks an enemy
    if state.team[idx].eidolon < 6 { return; }
    if action.action_type == ActionType::EnemyAttack { return; }

    let t = match target_idx.or_else(|| {
        state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
    }) {
        Some(t) => t,
        None    => return,
    };

    try_add_arcana(state, idx, t, 1.0, true);
}
