use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ─── Actual LC DEF UUID (matches Aventurine's lookup) ─────────────────────────
const LC_DEF_UUID: &str = "52566b38-915c-4220-ab0e-61438225704b";

// ─── Stack keys ───────────────────────────────────────────────────────────────
const TALENT_USED: &str = "gep_talent_used"; // 1 once Unyielding Will has fired
const FREEZE_ACC:  &str = "gep_freeze_acc";  // accumulator: 0.65/turn (1.0 with E1)
const A6_BONUS:    &str = "gep_a6_bonus";    // current flat ATK bonus from A6

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn get(state: &SimState, idx: usize, key: &str) -> f64 {
    state.team[idx].stacks.get(key).copied().unwrap_or(0.0)
}

fn set(state: &mut SimState, idx: usize, key: &'static str, v: f64) {
    state.team[idx].stacks.insert(key, v);
}

fn frozen_key(slot: usize) -> String {
    format!("gepard_frozen_{}", slot)
}

/// Current DEF (character + lightcone, after DEF%).
fn get_def(state: &SimState, idx: usize) -> f64 {
    let base = state.team[idx].base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(0.0)
        + state.team[idx].lightcone.base_stats.get(LC_DEF_UUID).copied().unwrap_or(0.0);
    base * (1.0 + state.team[idx].buffs.def_percent / 100.0)
}

/// Ult shield HP for a single ally (max level: 58.5% DEF + 1550, scaled by shield_effect).
fn ult_shield(state: &SimState, idx: usize) -> f64 {
    let def = get_def(state, idx);
    let base = 0.585 * def + 1550.0;
    base * (1.0 + state.team[idx].buffs.shield_effect / 100.0)
}

/// Recompute and update A6 flat ATK bonus (35% of current DEF).
fn refresh_a6(state: &mut SimState, idx: usize) {
    let new_bonus = get_def(state, idx) * 0.35;
    let old_bonus = get(state, idx, A6_BONUS);
    state.team[idx].buffs.atk_flat += new_bonus - old_bonus;
    set(state, idx, A6_BONUS, new_bonus);
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 100.0;

    // Minor traces: +22.4% Ice DMG, +18% Effect RES, +12.5% DEF
    state.team[idx].buffs.dmg_boost  += 22.4; // Ice DMG % treated as all-DMG for this char
    state.team[idx].buffs.effect_res += 18.0;
    state.team[idx].buffs.def_percent += 12.5;

    let eidolon = state.team[idx].eidolon;

    // E4: all allies +20% Effect RES
    if eidolon >= 4 {
        let n = state.team.len();
        for i in 0..n {
            state.team[i].buffs.effect_res += 20.0;
        }
    }

    // A6: initial ATK bonus from DEF (will be 0 on first call since buffs start at 0)
    refresh_a6(state, idx);

    // Initialise stacks
    for key in [TALENT_USED, FREEZE_ACC, A6_BONUS] {
        state.team[idx].stacks.insert(key, 0.0);
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.remove("_action_advance_pct");

    // A6: refresh flat ATK bonus at start of every turn
    refresh_a6(state, idx);
}

pub fn on_before_action(
    _state: &mut SimState,
    _idx: usize,
    _action: &mut ActionParams,
    _target_idx: Option<usize>,
) {}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    if action.action_type != ActionType::Skill { return; }

    let t = match target_idx.or_else(|| {
        state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
    }) {
        Some(t) => t,
        None    => return,
    };

    if state.enemies[t].as_ref().map_or(true, |e| e.hp <= 0.0) { return; }

    // Freeze accumulator: +1.0 with E1 (100%), +0.65 otherwise (65% base)
    let eidolon = state.team[idx].eidolon;
    let freeze_gain = if eidolon >= 1 { 1.0 } else { 0.65 };
    let acc = get(state, idx, FREEZE_ACC) + freeze_gain;
    if acc >= 1.0 {
        state.stacks.insert(frozen_key(t), 1.0);
        set(state, idx, FREEZE_ACC, acc - 1.0);
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Skill: enemy {} Frozen (1t)", t));
    } else {
        set(state, idx, FREEZE_ACC, acc);
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);

    let shield_hp = ult_shield(state, idx);

    let n = state.team.len();
    for i in 0..n {
        if !state.team[i].is_downed {
            state.team[i].shield = shield_hp;
        }
    }

    // Restore 5 energy after ult
    state.team[idx].energy = 5.0;

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Ult: Enduring Bulwark — shield {:.0} HP (all allies, 3t)",
        shield_hp,
    ));
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}

pub fn on_enemy_turn_start(state: &mut SimState, idx: usize, enemy_idx: usize) {
    // Frozen additional Ice DMG at start of the frozen enemy's turn
    let key = frozen_key(enemy_idx);
    let frozen = state.stacks.get(&key).copied().unwrap_or(0.0);
    if frozen > 0.0 && state.enemies[enemy_idx].as_ref().map_or(false, |e| e.hp > 0.0) {
        let frozen_action = ActionParams {
            action_type:      ActionType::TalentProc,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       0.80,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 0.0,
            inflicts_debuff:  false,
            is_ult_dmg:       false,
        };
        let dmg = {
            let m = &state.team[idx];
            state.enemies[enemy_idx].as_ref()
                .map(|e| damage::calculate_damage(m, e, &frozen_action))
                .unwrap_or(0.0)
        };
        if dmg > 0.0 {
            if let Some(e) = state.enemies[enemy_idx].as_mut() { e.hp -= dmg; }
            state.total_damage += dmg;
            if state.enemies[enemy_idx].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[enemy_idx] = None;
            }
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!("Frozen: {:.0} additional Ice DMG", dmg));
        }

        // Expire freeze after it fires
        state.stacks.remove(&key);
    }
}

pub fn on_enemy_action(state: &mut SimState, idx: usize, _enemy_idx: usize) {
    // Talent: Unyielding Will — survive a killing blow once per battle
    if get(state, idx, TALENT_USED) >= 1.0 { return; }
    if state.team[idx].hp > 0.0 { return; }

    let eidolon = state.team[idx].eidolon;

    // Restore HP to 35% Max HP; E6 adds 50% Max HP on top
    let restore_pct = if eidolon >= 6 { 0.85 } else { 0.35 };
    let max_hp = state.team[idx].max_hp;
    state.team[idx].hp = max_hp * restore_pct;
    state.team[idx].is_downed = false;
    set(state, idx, TALENT_USED, 1.0);

    // A4: restore energy to 100%
    state.team[idx].energy = state.team[idx].max_energy;

    // E6: 100% action advance
    if eidolon >= 6 {
        set(state, idx, "_action_advance_pct", 99.0);
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Unyielding Will: survived! HP restored to {:.0}/{:.0}{}",
        state.team[idx].hp, max_hp,
        if eidolon >= 6 { " | E6: action advance" } else { "" },
    ));
}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
