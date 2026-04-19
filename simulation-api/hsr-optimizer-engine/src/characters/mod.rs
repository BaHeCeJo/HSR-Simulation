use crate::ids;
use crate::models::{ActionParams, SimState};

pub mod acheron;
pub mod jiaoqiu;
pub mod pela;
pub mod silver_wolf;
pub mod aglaea;
pub mod argenti;
pub mod arlan;
pub mod ashveil;
pub mod asta;
pub mod aventurine;
pub mod bailu;
pub mod anaxa;
pub mod archer;
pub mod blade;
pub mod boothill;

// ─── Hook dispatch helpers ───────────────────────────────────────────────────

macro_rules! dispatch {
    ($fn_name:ident, $state:expr, $idx:expr) => {{
        let kit_id = $state.team[$idx].kit_id.clone();
        match kit_id.as_str() {
            ids::ACHERON_ID     => acheron::$fn_name($state, $idx),
            ids::JIAOQIU_ID     => jiaoqiu::$fn_name($state, $idx),
            ids::PELA_ID        => pela::$fn_name($state, $idx),
            ids::SILVER_WOLF_ID => silver_wolf::$fn_name($state, $idx),
            ids::AGLAEA_ID      => aglaea::$fn_name($state, $idx),
            ids::ARGENTI_ID     => argenti::$fn_name($state, $idx),
            ids::ARLAN_ID       => arlan::$fn_name($state, $idx),
            ids::ASHVEIL_ID     => ashveil::$fn_name($state, $idx),
            ids::ASTA_ID        => asta::$fn_name($state, $idx),
            ids::AVENTURINE_ID  => aventurine::$fn_name($state, $idx),
            ids::BAILU_ID       => bailu::$fn_name($state, $idx),
            ids::ANAXA_ID       => anaxa::$fn_name($state, $idx),
            ids::ARCHER_ID      => archer::$fn_name($state, $idx),
            ids::BLADE_ID       => blade::$fn_name($state, $idx),
            ids::BOOTHILL_ID    => boothill::$fn_name($state, $idx),
            _                   => {}
        }
    }};
}

pub fn dispatch_on_battle_start(state: &mut SimState, idx: usize) {
    dispatch!(on_battle_start, state, idx);
}

pub fn dispatch_on_turn_start(state: &mut SimState, idx: usize) {
    dispatch!(on_turn_start, state, idx);
}

pub fn dispatch_on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    let kit_id = state.team[idx].kit_id.clone();
    match kit_id.as_str() {
        ids::ACHERON_ID     => acheron::on_before_action(state, idx, action, target_idx),
        ids::JIAOQIU_ID     => jiaoqiu::on_before_action(state, idx, action, target_idx),
        ids::PELA_ID        => pela::on_before_action(state, idx, action, target_idx),
        ids::SILVER_WOLF_ID => silver_wolf::on_before_action(state, idx, action, target_idx),
        ids::AGLAEA_ID      => aglaea::on_before_action(state, idx, action, target_idx),
        ids::ARGENTI_ID     => argenti::on_before_action(state, idx, action, target_idx),
        ids::ARLAN_ID       => arlan::on_before_action(state, idx, action, target_idx),
        ids::ASHVEIL_ID     => ashveil::on_before_action(state, idx, action, target_idx),
        ids::ASTA_ID        => asta::on_before_action(state, idx, action, target_idx),
        ids::AVENTURINE_ID  => aventurine::on_before_action(state, idx, action, target_idx),
        ids::BAILU_ID       => bailu::on_before_action(state, idx, action, target_idx),
        ids::ANAXA_ID       => anaxa::on_before_action(state, idx, action, target_idx),
        ids::ARCHER_ID      => archer::on_before_action(state, idx, action, target_idx),
        ids::BLADE_ID       => blade::on_before_action(state, idx, action, target_idx),
        ids::BOOTHILL_ID    => boothill::on_before_action(state, idx, action, target_idx),
        _                   => {}
    }
}

pub fn dispatch_on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    let kit_id = state.team[idx].kit_id.clone();
    match kit_id.as_str() {
        ids::ACHERON_ID     => acheron::on_after_action(state, idx, action, target_idx),
        ids::JIAOQIU_ID     => jiaoqiu::on_after_action(state, idx, action, target_idx),
        ids::PELA_ID        => pela::on_after_action(state, idx, action, target_idx),
        ids::SILVER_WOLF_ID => silver_wolf::on_after_action(state, idx, action, target_idx),
        ids::AGLAEA_ID      => aglaea::on_after_action(state, idx, action, target_idx),
        ids::ARGENTI_ID     => argenti::on_after_action(state, idx, action, target_idx),
        ids::ARLAN_ID       => arlan::on_after_action(state, idx, action, target_idx),
        ids::ASHVEIL_ID     => ashveil::on_after_action(state, idx, action, target_idx),
        ids::ASTA_ID        => asta::on_after_action(state, idx, action, target_idx),
        ids::AVENTURINE_ID  => aventurine::on_after_action(state, idx, action, target_idx),
        ids::BAILU_ID       => bailu::on_after_action(state, idx, action, target_idx),
        ids::ANAXA_ID       => anaxa::on_after_action(state, idx, action, target_idx),
        ids::ARCHER_ID      => archer::on_after_action(state, idx, action, target_idx),
        ids::BLADE_ID       => blade::on_after_action(state, idx, action, target_idx),
        ids::BOOTHILL_ID    => boothill::on_after_action(state, idx, action, target_idx),
        _                   => {}
    }
}

pub fn dispatch_on_ult(state: &mut SimState, idx: usize) {
    dispatch!(on_ult, state, idx);
}

/// Called on every team member when ANY debuff is applied to an enemy.
pub fn dispatch_on_global_debuff(state: &mut SimState, source_idx: usize, enemy_idx: usize) {
    // Iterate over all team members (not just the source)
    for i in 0..state.team.len() {
        let kit_id = state.team[i].kit_id.clone();
        match kit_id.as_str() {
            ids::ACHERON_ID     => acheron::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::JIAOQIU_ID     => jiaoqiu::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::PELA_ID        => pela::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::SILVER_WOLF_ID => silver_wolf::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::AGLAEA_ID      => aglaea::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::ARGENTI_ID     => argenti::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::ARLAN_ID       => arlan::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::ASHVEIL_ID     => ashveil::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::ASTA_ID        => asta::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::AVENTURINE_ID  => aventurine::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::BAILU_ID       => bailu::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::ANAXA_ID       => anaxa::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::ARCHER_ID      => archer::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::BLADE_ID       => blade::on_global_debuff(state, i, source_idx, enemy_idx),
            ids::BOOTHILL_ID    => boothill::on_global_debuff(state, i, source_idx, enemy_idx),
            _                   => {}
        }
    }
}

/// Called on every team member at the start of an enemy's turn.
pub fn dispatch_on_enemy_turn_start(state: &mut SimState, enemy_idx: usize) {
    for i in 0..state.team.len() {
        let kit_id = state.team[i].kit_id.clone();
        match kit_id.as_str() {
            ids::ACHERON_ID     => acheron::on_enemy_turn_start(state, i, enemy_idx),
            ids::JIAOQIU_ID     => jiaoqiu::on_enemy_turn_start(state, i, enemy_idx),
            ids::PELA_ID        => pela::on_enemy_turn_start(state, i, enemy_idx),
            ids::SILVER_WOLF_ID => silver_wolf::on_enemy_turn_start(state, i, enemy_idx),
            ids::AGLAEA_ID      => aglaea::on_enemy_turn_start(state, i, enemy_idx),
            ids::ARGENTI_ID     => argenti::on_enemy_turn_start(state, i, enemy_idx),
            ids::ARLAN_ID       => arlan::on_enemy_turn_start(state, i, enemy_idx),
            ids::ASHVEIL_ID     => ashveil::on_enemy_turn_start(state, i, enemy_idx),
            ids::ASTA_ID        => asta::on_enemy_turn_start(state, i, enemy_idx),
            ids::AVENTURINE_ID  => aventurine::on_enemy_turn_start(state, i, enemy_idx),
            ids::BAILU_ID       => bailu::on_enemy_turn_start(state, i, enemy_idx),
            ids::ANAXA_ID       => anaxa::on_enemy_turn_start(state, i, enemy_idx),
            ids::ARCHER_ID      => archer::on_enemy_turn_start(state, i, enemy_idx),
            ids::BLADE_ID       => blade::on_enemy_turn_start(state, i, enemy_idx),
            ids::BOOTHILL_ID    => boothill::on_enemy_turn_start(state, i, enemy_idx),
            _                   => {}
        }
    }
}

/// Called on every team member after an enemy acts.
pub fn dispatch_on_enemy_action(state: &mut SimState, enemy_idx: usize) {
    for i in 0..state.team.len() {
        let kit_id = state.team[i].kit_id.clone();
        match kit_id.as_str() {
            ids::ACHERON_ID     => acheron::on_enemy_action(state, i, enemy_idx),
            ids::JIAOQIU_ID     => jiaoqiu::on_enemy_action(state, i, enemy_idx),
            ids::PELA_ID        => pela::on_enemy_action(state, i, enemy_idx),
            ids::SILVER_WOLF_ID => silver_wolf::on_enemy_action(state, i, enemy_idx),
            ids::AGLAEA_ID      => aglaea::on_enemy_action(state, i, enemy_idx),
            ids::ARGENTI_ID     => argenti::on_enemy_action(state, i, enemy_idx),
            ids::ARLAN_ID       => arlan::on_enemy_action(state, i, enemy_idx),
            ids::ASHVEIL_ID     => ashveil::on_enemy_action(state, i, enemy_idx),
            ids::ASTA_ID        => asta::on_enemy_action(state, i, enemy_idx),
            ids::AVENTURINE_ID  => aventurine::on_enemy_action(state, i, enemy_idx),
            ids::BAILU_ID       => bailu::on_enemy_action(state, i, enemy_idx),
            ids::ANAXA_ID       => anaxa::on_enemy_action(state, i, enemy_idx),
            ids::ARCHER_ID      => archer::on_enemy_action(state, i, enemy_idx),
            ids::BLADE_ID       => blade::on_enemy_action(state, i, enemy_idx),
            ids::BOOTHILL_ID    => boothill::on_enemy_action(state, i, enemy_idx),
            _                   => {}
        }
    }
}

/// Called on every team member when an enemy is Weakness Broken.
pub fn dispatch_on_break(state: &mut SimState, idx: usize, enemy_slot: usize) {
    let kit_id = state.team[idx].kit_id.clone();
    match kit_id.as_str() {
        ids::SILVER_WOLF_ID => silver_wolf::on_break(state, idx, enemy_slot),
        ids::BOOTHILL_ID    => boothill::on_break(state, idx, enemy_slot),
        _                   => {}
    }
}

/// Called on every team member after an ally acts.
pub fn dispatch_on_ally_action(
    state: &mut SimState,
    source_idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    for i in 0..state.team.len() {
        if i == source_idx { continue; } // don't call on yourself
        let kit_id = state.team[i].kit_id.clone();
        match kit_id.as_str() {
            ids::ACHERON_ID     => acheron::on_ally_action(state, i, source_idx, action, target_idx),
            ids::JIAOQIU_ID     => jiaoqiu::on_ally_action(state, i, source_idx, action, target_idx),
            ids::PELA_ID        => pela::on_ally_action(state, i, source_idx, action, target_idx),
            ids::SILVER_WOLF_ID => silver_wolf::on_ally_action(state, i, source_idx, action, target_idx),
            ids::AGLAEA_ID      => aglaea::on_ally_action(state, i, source_idx, action, target_idx),
            ids::ARGENTI_ID     => argenti::on_ally_action(state, i, source_idx, action, target_idx),
            ids::ARLAN_ID       => arlan::on_ally_action(state, i, source_idx, action, target_idx),
            ids::ASHVEIL_ID     => ashveil::on_ally_action(state, i, source_idx, action, target_idx),
            ids::ASTA_ID        => asta::on_ally_action(state, i, source_idx, action, target_idx),
            ids::AVENTURINE_ID  => aventurine::on_ally_action(state, i, source_idx, action, target_idx),
            ids::BAILU_ID       => bailu::on_ally_action(state, i, source_idx, action, target_idx),
            ids::ANAXA_ID       => anaxa::on_ally_action(state, i, source_idx, action, target_idx),
            ids::ARCHER_ID      => archer::on_ally_action(state, i, source_idx, action, target_idx),
            ids::BLADE_ID       => blade::on_ally_action(state, i, source_idx, action, target_idx),
            ids::BOOTHILL_ID    => boothill::on_ally_action(state, i, source_idx, action, target_idx),
            _                   => {}
        }
    }
}
