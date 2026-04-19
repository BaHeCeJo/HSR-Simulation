use crate::effects;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

const TALENT_TRIGGERS_KEY: &str = "bailu_talent_triggers";
pub const REVIVES_KEY: &str     = "bailu_ko_revives";
const E2_TURNS_KEY: &str        = "bailu_e2_turns";

fn apply_invigoration(state: &mut SimState, member_idx: usize) {
    let existing_dur = state.team[member_idx].active_buffs
        .get("bailu_invigoration").map(|b| b.duration).unwrap_or(0);
    let new_dur = if existing_dur > 0 { existing_dur + 1 } else { 2 };
    effects::apply_member_buff(&mut state.team[member_idx], "bailu_invigoration", StatusEffect {
        duration: new_dur, value: 1.0,
        stat: Some("Invigoration".to_string()), effects: vec![],
    });
}

fn do_heal(state: &mut SimState, bailu_max_hp: f64, target_idx: usize, pct: f64, flat: f64) {
    let e2_active = state.stacks.get(E2_TURNS_KEY).copied().unwrap_or(0.0) > 0.0;
    let mut amount = pct * bailu_max_hp + flat;
    if e2_active { amount *= 1.15; }
    let max = state.team[target_idx].max_hp;
    state.team[target_idx].hp = (state.team[target_idx].hp + amount).min(max);
}

fn apply_e4(state: &mut SimState, bailu_idx: usize, target_idx: usize) {
    if state.team[bailu_idx].eidolon < 4 { return; }
    let kit_id = state.team[target_idx].kit_id.clone();
    let key = format!("bailu_e4_{}", kit_id);
    let stacks = state.stacks.get(&key).copied().unwrap_or(0.0);
    if stacks >= 3.0 { return; }
    state.stacks.insert(key, stacks + 1.0);
    state.team[target_idx].buffs.dmg_boost += 10.0;
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy        = 100.0;
    state.team[idx].buffs.hp_percent  += 28.0; // minor trace: HP +28%
    state.team[idx].buffs.def_percent += 22.5; // minor trace: DEF +22.5%
    state.team[idx].buffs.effect_res  += 10.0; // minor trace: Effect RES +10%

    let eidolon = state.team[idx].eidolon;
    state.stacks.insert(TALENT_TRIGGERS_KEY.to_string(), 4.0); // A4: +1 trigger
    state.stacks.insert(REVIVES_KEY.to_string(), if eidolon >= 6 { 2.0 } else { 1.0 });
    state.stacks.insert(E2_TURNS_KEY.to_string(), 0.0);

    // Technique: apply Invigoration (2 turns) to all alive allies at battle start
    for i in 0..state.team.len() {
        if !state.team[i].is_downed {
            effects::apply_member_buff(&mut state.team[i], "bailu_invigoration", StatusEffect {
                duration: 2, value: 1.0,
                stat: Some("Invigoration".to_string()), effects: vec![],
            });
        }
    }
}

pub fn on_turn_start(state: &mut SimState, _idx: usize) {
    let turns = state.stacks.get(E2_TURNS_KEY).copied().unwrap_or(0.0);
    if turns > 0.0 {
        state.stacks.insert(E2_TURNS_KEY.to_string(), turns - 1.0);
    }
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
    _target_idx: Option<usize>,
) {
    if action.action_type != ActionType::Skill { return; }

    let bailu_max_hp = state.team[idx].max_hp;

    // 1st heal: lowest-HP alive ally
    let t1 = (0..state.team.len())
        .filter(|&i| !state.team[i].is_downed)
        .min_by(|&a, &b| state.team[a].hp.partial_cmp(&state.team[b].hp).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(idx);
    do_heal(state, bailu_max_hp, t1, 0.117, 312.0);
    apply_e4(state, idx, t1);

    // 2nd heal: ×0.85 decay → first alive ally
    let base_heal = 0.117 * bailu_max_hp + 312.0;
    let t2 = (0..state.team.len()).find(|&i| !state.team[i].is_downed).unwrap_or(idx);
    let h2_pct = base_heal * 0.85 / bailu_max_hp.max(1.0);
    do_heal(state, bailu_max_hp, t2, h2_pct, 0.0);
    apply_e4(state, idx, t2);

    // 3rd heal: ×0.7225 → second alive ally (or same)
    let t3 = (0..state.team.len()).filter(|&i| !state.team[i].is_downed).nth(1).unwrap_or(t2);
    let h3_pct = base_heal * 0.7225 / bailu_max_hp.max(1.0);
    do_heal(state, bailu_max_hp, t3, h3_pct, 0.0);
    apply_e4(state, idx, t3);
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;
    if eidolon >= 2 {
        state.stacks.insert(E2_TURNS_KEY.to_string(), 2.0);
    }

    let bailu_max_hp = state.team[idx].max_hp;

    // Heal all alive allies: 13.5% MaxHP + 360
    for i in 0..state.team.len() {
        if !state.team[i].is_downed {
            do_heal(state, bailu_max_hp, i, 0.135, 360.0);
        }
    }

    // Apply/extend Invigoration to all alive allies
    for i in 0..state.team.len() {
        if !state.team[i].is_downed {
            apply_invigoration(state, i);
        }
    }

    // Reset talent trigger counter
    state.stacks.insert(TALENT_TRIGGERS_KEY.to_string(), 4.0); // A4: +1 trigger

    let name = state.team[idx].name.clone();
    state.add_log(&name, "Felicitous Thunderleap: all allies healed + Invigoration.".to_string());
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_enemy_action(state: &mut SimState, idx: usize, _enemy_idx: usize) {
    let triggers = state.stacks.get(TALENT_TRIGGERS_KEY).copied().unwrap_or(0.0);
    if triggers <= 0.0 { return; }

    let bailu_max_hp = state.team[idx].max_hp;

    // Heal the lowest-HP Invigorated alive ally
    let target = (0..state.team.len())
        .filter(|&i| {
            !state.team[i].is_downed
                && state.team[i].active_buffs.contains_key("bailu_invigoration")
        })
        .min_by(|&a, &b| {
            state.team[a].hp.partial_cmp(&state.team[b].hp).unwrap_or(std::cmp::Ordering::Equal)
        });

    if let Some(t) = target {
        do_heal(state, bailu_max_hp, t, 0.054, 144.0);
        state.stacks.insert(TALENT_TRIGGERS_KEY.to_string(), triggers - 1.0);
    }
}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
