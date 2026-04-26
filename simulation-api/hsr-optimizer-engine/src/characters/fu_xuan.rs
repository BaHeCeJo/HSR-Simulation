use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ─── Stack keys ───────────────────────────────────────────────────────────────
const MOP_TURNS:   &str = "fx_mop_turns";   // MoP turns remaining (ticks at Fu Xuan's turn start)
const MOP_APPLIED: &str = "fx_mop_applied"; // 1 when Knowledge is active on allies
const HEAL_TRIG:   &str = "fx_heal_trig";   // HP Restore trigger count (1 default, max 2)

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn get(state: &SimState, idx: usize, key: &str) -> f64 {
    state.team[idx].stacks.get(key).copied().unwrap_or(0.0)
}

fn set(state: &mut SimState, idx: usize, key: &'static str, v: f64) {
    state.team[idx].stacks.insert(key, v);
}

/// Remove Knowledge CRIT bonuses from all allies.
fn remove_knowledge_all(state: &mut SimState, fx_idx: usize) {
    let eidolon = state.team[fx_idx].eidolon;
    for i in 0..state.team.len() {
        state.team[i].buffs.crit_rate -= 12.0;
        if eidolon >= 1 { state.team[i].buffs.crit_dmg -= 30.0; }
    }
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 135.0;

    // Minor traces: +18.7% CRIT Rate, +10% Effect RES, +18% HP
    state.team[idx].buffs.crit_rate  += 18.7;
    state.team[idx].buffs.effect_res += 10.0;

    // +18% HP: apply retroactively after max_hp is already finalised.
    // base_hp * 0.18 = max_hp * 0.18 / (1 + buffs.hp_percent/100), ensuring the flat bonus
    // equals 18% of base HP regardless of existing relic HP%.
    let existing_pct = state.team[idx].buffs.hp_percent;
    let hp_bonus = state.team[idx].max_hp * 0.18 / (1.0 + existing_pct / 100.0);
    state.team[idx].max_hp += hp_bonus;
    state.team[idx].hp = state.team[idx].max_hp;
    state.team[idx].buffs.hp_percent += 18.0; // keeps damage formula in sync

    // Talent: Misfortune Avoidance — 18% incoming DMG reduction for all allies (passive)
    let n = state.team.len();
    for i in 0..n {
        state.team[i].buffs.incoming_dmg_reduction += 18.0;
    }

    // Talent: 1 HP Restore trigger by default
    set(state, idx, HEAL_TRIG, 1.0);

    for key in [MOP_TURNS, MOP_APPLIED] {
        state.team[idx].stacks.insert(key, 0.0);
    }

    // E6: initialise HP loss tally and per-ally HP snapshots
    state.stacks.insert("fuxuan_e6_tally".to_string(), 0.0);
    let hps: Vec<f64> = state.team.iter().map(|m| m.hp).collect();
    for (i, &hp) in hps.iter().enumerate() {
        state.stacks.insert(format!("fuxuan_hp_snap_{}", i), hp);
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // ── Matrix of Prescience duration tick ───────────────────────────────────
    let mop_turns   = get(state, idx, MOP_TURNS);
    let mop_applied = get(state, idx, MOP_APPLIED);

    if mop_turns > 0.0 {
        if mop_applied < 1.0 {
            // First Fu Xuan turn after Skill: apply her OWN Knowledge buff here so it
            // survives the snapshot restore (on_turn_start runs before the snapshot is taken).
            let eidolon = state.team[idx].eidolon;
            state.team[idx].buffs.crit_rate += 12.0;
            if eidolon >= 1 { state.team[idx].buffs.crit_dmg += 30.0; }
            set(state, idx, MOP_APPLIED, 1.0);
            // Don't tick yet on first application turn
        } else {
            let new_turns = mop_turns - 1.0;
            set(state, idx, MOP_TURNS, new_turns);
            if new_turns <= 0.0 {
                remove_knowledge_all(state, idx);
                set(state, idx, MOP_APPLIED, 0.0);
                let name = state.team[idx].name.clone();
                state.add_log(&name, "Matrix of Prescience expired".to_string());
            }
        }
    }

    // ── Talent HP Restore (proactive check at turn start) ────────────────────
    check_hp_restore(state, idx);
}

fn check_hp_restore(state: &mut SimState, idx: usize) {
    let hp      = state.team[idx].hp;
    let max_hp  = state.team[idx].max_hp;
    let trig    = get(state, idx, HEAL_TRIG);
    if hp > 0.0 && hp <= max_hp * 0.50 && trig >= 1.0 {
        let missing = max_hp - hp;
        let heal    = missing * 0.90;
        state.team[idx].hp = (hp + heal).min(max_hp);
        set(state, idx, HEAL_TRIG, trig - 1.0);
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!(
            "HP Restore: +{:.0} HP ({:.0}/{:.0}) | {:.0} trigger(s) left",
            heal, state.team[idx].hp, max_hp, trig - 1.0
        ));
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    // Basic ATK is HP-scaling (50% Max HP)
    if action.action_type == ActionType::Basic {
        action.multiplier       = 0.50;
        action.scaling_stat_id  = ids::CHAR_HP_ID.to_string();
    }
    let _ = (state, idx); // suppress unused warnings
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    _target_idx: Option<usize>,
) {
    if action.action_type != ActionType::Skill { return; }

    let mop_already_active = get(state, idx, MOP_APPLIED) >= 1.0
        || get(state, idx, MOP_TURNS) > 0.0;

    if mop_already_active {
        // Refresh duration; Knowledge already applied, no re-application needed
        set(state, idx, MOP_TURNS, 3.0);
        // A2: +20 extra energy when Skill is used while MoP is active
        let err = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
        let bonus = 20.0 * err;
        let max_e = state.team[idx].max_energy;
        state.team[idx].energy = (state.team[idx].energy + bonus).min(max_e);
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Matrix of Prescience refreshed (3t) | A2 +{:.0} energy", bonus));
    } else {
        // Fresh MoP activation.
        // Apply Knowledge to all OTHER allies now (snapshot restore will NOT affect them).
        // Fu Xuan's own Knowledge is deferred to on_turn_start to survive the snapshot restore.
        let eidolon = state.team[idx].eidolon;
        let n = state.team.len();
        for i in 0..n {
            if i == idx { continue; }
            state.team[i].buffs.crit_rate += 12.0;
            if eidolon >= 1 { state.team[i].buffs.crit_dmg += 30.0; }
        }
        set(state, idx, MOP_TURNS, 3.0);
        // MOP_APPLIED left at 0; set to 1 in next on_turn_start when Fu Xuan's buff is applied
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!(
            "Matrix of Prescience activated (3t) | Knowledge: +12% CR{}",
            if eidolon >= 1 { " +30% CD" } else { "" }
        ));
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);

    let eidolon   = state.team[idx].eidolon;
    let fx_max_hp = state.team[idx].max_hp;

    // E6: compute ult DMG boost from HP loss tally, then reset tally
    let e6_extra_dmg = if eidolon >= 6 {
        let tally  = state.stacks.get("fuxuan_e6_tally").copied().unwrap_or(0.0);
        let capped = tally.min(fx_max_hp * 1.2);
        state.stacks.insert("fuxuan_e6_tally".to_string(), 0.0);
        // Reinit HP snapshots after reset so future tally starts from 0
        let hps: Vec<f64> = state.team.iter().map(|m| m.hp).collect();
        for (i, &hp) in hps.iter().enumerate() {
            state.stacks.insert(format!("fuxuan_hp_snap_{}", i), hp);
        }
        capped * 2.0
    } else {
        0.0
    };

    // AoE ult: 100% Max HP to all enemies (toughness skipped — see APPROXIMATIONS.md)
    let name = state.team[idx].name.clone();
    let mut total_dmg = 0.0;
    let enemy_count = state.enemies.iter()
        .filter(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
        .count();

    for slot in 0..state.enemies.len() {
        if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }

        let ult_action = ActionParams {
            action_type:      ActionType::Ultimate,
            scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
            multiplier:       1.00,
            extra_multiplier: 0.0,
            extra_dmg:        e6_extra_dmg,
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

    state.add_log(&name, format!(
        "Ult: {:.0} DMG ({} targets){}",
        total_dmg,
        enemy_count,
        if e6_extra_dmg > 0.0 { format!(" | E6 +{:.0} flat", e6_extra_dmg) } else { String::new() }
    ));

    // +1 HP Restore trigger (max 2)
    let trig = get(state, idx, HEAL_TRIG);
    set(state, idx, HEAL_TRIG, (trig + 1.0).min(2.0));

    // A4: heal all OTHER allies by 5% Fu Xuan Max HP + 133
    let heal_amt = fx_max_hp * 0.05 + 133.0;
    let n = state.team.len();
    for i in 0..n {
        if i == idx { continue; }
        let new_hp = (state.team[i].hp + heal_amt).min(state.team[i].max_hp);
        state.team[i].hp = new_hp;
    }
    if n > 1 {
        state.add_log(&name, format!("A4: healed allies +{:.0} HP each", heal_amt));
    }
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_enemy_action(state: &mut SimState, idx: usize, _enemy_idx: usize) {
    let eidolon = state.team[idx].eidolon;

    // E4: +5 energy when allies under MoP are attacked
    if eidolon >= 4 && get(state, idx, MOP_APPLIED) >= 1.0 {
        let max_e = state.team[idx].max_energy;
        state.team[idx].energy = (state.team[idx].energy + 5.0).min(max_e);
    }

    // E6: track HP loss across all allies (compare vs last snapshot)
    let n = state.team.len();
    let mut delta_sum = 0.0;
    for i in 0..n {
        let snap_key = format!("fuxuan_hp_snap_{}", i);
        let prev  = state.stacks.get(&snap_key).copied().unwrap_or(state.team[i].hp);
        let delta = (prev - state.team[i].hp).max(0.0);
        delta_sum += delta;
        state.stacks.insert(snap_key, state.team[i].hp);
    }
    if delta_sum > 0.0 {
        *state.stacks.entry("fuxuan_e6_tally".to_string()).or_insert(0.0) += delta_sum;
    }

    // HP Restore reactive trigger after taking enemy damage
    check_hp_restore(state, idx);
}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
