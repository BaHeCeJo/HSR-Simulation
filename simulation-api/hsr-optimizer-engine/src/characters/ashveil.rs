use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

const CHARGE_KEY:       &str = "ashveil_charge";
const GLUTTONY_KEY:     &str = "ashveil_gluttony";
const GLUTTONY_TOTAL:   &str = "ashveil_gluttony_total";
const BAIT_DEF_APPLIED: &str = "ashveil_bait_def_applied";
const E4_TURNS:         &str = "ashveil_e4_turns";
const BAIT_HIT_FLAG:    &str = "ashveil_skill_hit_bait";

fn get_charge(state: &SimState) -> f64 {
    state.stacks.get(CHARGE_KEY).copied().unwrap_or(0.0)
}

fn get_gluttony(state: &SimState) -> f64 {
    state.stacks.get(GLUTTONY_KEY).copied().unwrap_or(0.0)
}

fn max_gluttony(eidolon: i32) -> f64 {
    if eidolon >= 2 { 18.0 } else { 12.0 }
}

fn add_gluttony(state: &mut SimState, eidolon: i32, amount: f64) {
    let max = max_gluttony(eidolon);
    let before = get_gluttony(state);
    let after = (before + amount).min(max);
    state.stacks.insert(GLUTTONY_KEY.to_string(), after);
    let gained = (after - before).max(0.0);
    if gained > 0.0 {
        let total = (state.stacks.get(GLUTTONY_TOTAL).copied().unwrap_or(0.0) + gained).min(30.0);
        state.stacks.insert(GLUTTONY_TOTAL.to_string(), total);
    }
}

/// Returns the slot index of the enemy currently marked as Bait, if any.
fn get_bait_idx(state: &SimState) -> Option<usize> {
    state.enemies.iter().position(|s| {
        s.as_ref().map_or(false, |e| e.hp > 0.0 && e.active_debuffs.contains_key("bait"))
    })
}

/// Move Bait to the alive enemy with lowest HP.
fn move_bait_to_lowest(state: &mut SimState, eidolon: i32) -> Option<usize> {
    let target = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| s.as_ref().filter(|e| e.hp > 0.0).map(|e| (i, e.hp)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i);
    if let Some(t) = target {
        apply_bait(state, t, eidolon);
    }
    target
}

fn apply_bait(state: &mut SimState, target_idx: usize, eidolon: i32) {
    // Remove bait from previous target
    for s in state.enemies.iter_mut() {
        if let Some(e) = s.as_mut() {
            e.active_debuffs.remove("bait");
        }
    }
    // Apply to new target
    if let Some(e) = state.enemies[target_idx].as_mut() {
        effects::apply_enemy_debuff(e, "bait", StatusEffect {
            duration: 9999, value: 1.0, stat: Some("Bait".to_string()), effects: vec![],
        });
    }

    // On first Bait: all allies +40 DEF ignore
    if state.stacks.get(BAIT_DEF_APPLIED).copied().unwrap_or(0.0) < 1.0 {
        state.stacks.insert(BAIT_DEF_APPLIED.to_string(), 1.0);
        for m in state.team.iter_mut() {
            m.buffs.def_ignore += 40.0;
        }
        // E6: all enemies -20% All-Type RES
        if eidolon >= 6 {
            for slot in state.enemies.iter_mut() {
                if let Some(e) = slot.as_mut() {
                    e.resistance = (e.resistance - 0.20).max(-1.0);
                    for res in e.elemental_res.values_mut() {
                        *res = (*res - 0.20).max(-1.0);
                    }
                }
            }
        }
    }
}

fn fire_fup(state: &mut SimState, idx: usize, bait_target: usize, consume_charge: bool) -> bool {
    let eidolon = state.team[idx].eidolon;

    // Relic: reset Ashblazing stacks for this new FUA sequence.
    crate::relics::on_follow_up_start(&mut state.team, idx);

    let mut fup_member = state.team[idx].clone();

    // E4: ATK boost if active
    if state.stacks.get(E4_TURNS).copied().unwrap_or(0.0) > 0.0 {
        fup_member.buffs.atk_percent += 40.0;
    }

    // A4: FUP DMG +80% + 10% per Gluttony stack; A6: +80% CRIT DMG for FUPs
    fup_member.buffs.dmg_boost += 80.0 + get_gluttony(state) * 10.0;
    fup_member.buffs.crit_dmg  += 80.0;

    // E6: DMG from lifetime Gluttony
    if eidolon >= 6 {
        fup_member.buffs.dmg_boost += state.stacks.get(GLUTTONY_TOTAL).copied().unwrap_or(0.0) * 4.0;
    }

    let fup_action = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       2.0,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 5.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let dmg = state.enemies[bait_target].as_ref()
        .map(|e| damage::calculate_damage(&fup_member, e, &fup_action))
        .unwrap_or(0.0);
    if dmg > 0.0 {
        if let Some(e) = state.enemies[bait_target].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
    }

    if consume_charge {
        let ch = get_charge(state);
        state.stacks.insert(CHARGE_KEY.to_string(), (ch - 1.0).max(0.0));
    }

    // Relic: 1 hit per FUP call → increment Ashblazing stack; set Wind-Soaring window.
    crate::relics::on_follow_up_hit(&mut state.team, idx);
    crate::relics::on_follow_up_end(&mut state.team, idx);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Ashveil FUP: {:.0} DMG (Charge={}, Gluttony={})",
        dmg, get_charge(state), get_gluttony(state)));

    let killed = state.enemies[bait_target].as_ref().map_or(false, |e| e.hp <= 0.0);
    if killed { state.enemies[bait_target] = None; }
    killed
}

/// Enhanced FUP chain triggered by ult (no charge cost; gluttony consumption loop).
fn launch_enhanced_fup(state: &mut SimState, idx: usize) {
    let eidolon = state.team[idx].eidolon;

    let mut bait = match get_bait_idx(state) {
        Some(t) => t,
        None    => match move_bait_to_lowest(state, eidolon) { Some(t) => t, None => return },
    };

    let killed = fire_fup(state, idx, bait, false);
    if killed {
        add_gluttony(state, eidolon, 1.0); // A2 kill bonus
        match move_bait_to_lowest(state, eidolon) {
            Some(t) => bait = t,
            None    => return,
        }
    }

    // Gluttony consumption loop: consume 4 → deal FUP, repeat
    let mut consumed = 0.0f64;
    loop {
        if get_gluttony(state) < 4.0 { break; }
        let current = match get_bait_idx(state)
            .or_else(|| move_bait_to_lowest(state, eidolon))
        {
            Some(t) => t,
            None    => break,
        };
        state.stacks.insert(GLUTTONY_KEY.to_string(), get_gluttony(state) - 4.0);
        consumed += 4.0;
        let chain_killed = fire_fup(state, idx, current, false);
        if chain_killed {
            add_gluttony(state, eidolon, 1.0);
            match move_bait_to_lowest(state, eidolon) {
                Some(t) => { bait = t; }
                None    => break,
            }
        }
    }

    // E2: refund 35% of consumed Gluttony
    if eidolon >= 2 && consumed > 0.0 {
        let refund = (consumed * 0.35).floor();
        if refund > 0.0 { add_gluttony(state, eidolon, refund); }
    }
    let _ = bait;
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy     = 150.0;
    state.team[idx].buffs.atk_percent += 10.0; // minor trace
    state.team[idx].buffs.dmg_boost   += 14.4; // minor trace: Lightning DMG
    state.team[idx].buffs.crit_dmg    += 37.3; // minor trace

    state.stacks.insert(CHARGE_KEY.to_string(), 2.0);
    state.stacks.insert(GLUTTONY_KEY.to_string(), 0.0);
    state.stacks.insert(GLUTTONY_TOTAL.to_string(), 0.0);
    state.stacks.insert(BAIT_DEF_APPLIED.to_string(), 0.0);
    state.stacks.insert(E4_TURNS.to_string(), 0.0);

    // A6: all allies +40% CRIT DMG
    for m in state.team.iter_mut() {
        m.buffs.crit_dmg += 40.0;
    }

    // E1: all enemies +24% vulnerability
    let eidolon = state.team[idx].eidolon;
    if eidolon >= 1 {
        for slot in state.enemies.iter_mut() {
            if let Some(e) = slot.as_mut() {
                e.vulnerability += 24.0;
                state.stacks.insert(format!("ashveil_e1_{}", e.instance_id), 24.0);
            }
        }
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // E4: decrement ATK boost counter
    let turns = state.stacks.get(E4_TURNS).copied().unwrap_or(0.0);
    if turns > 0.0 {
        state.stacks.insert(E4_TURNS.to_string(), turns - 1.0);
    }

    // E1: update vulnerability for enemies near 50% HP threshold
    let eidolon = state.team[idx].eidolon;
    if eidolon >= 1 {
        for slot in state.enemies.iter_mut() {
            if let Some(e) = slot.as_mut() {
                let key = format!("ashveil_e1_{}", e.instance_id);
                let current = state.stacks.get(&key).copied().unwrap_or(0.0);
                let next = if e.max_hp > 0.0 && e.hp / e.max_hp <= 0.5 { 36.0 } else { 24.0 };
                e.vulnerability += next - current;
                state.stacks.insert(key, next);
            }
        }
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    let eidolon = state.team[idx].eidolon;

    // E4: ATK boost if active
    if state.stacks.get(E4_TURNS).copied().unwrap_or(0.0) > 0.0 {
        state.team[idx].buffs.atk_percent += 40.0;
    }

    // E6: DMG boost from lifetime Gluttony
    if eidolon >= 6 {
        state.team[idx].buffs.dmg_boost +=
            state.stacks.get(GLUTTONY_TOTAL).copied().unwrap_or(0.0) * 4.0;
    }

    // Skill on existing Bait: boost multiplier to 300% (200% + 100% bonus)
    if action.action_type == ActionType::Skill {
        if let Some(t) = target_idx {
            let is_bait = state.enemies[t].as_ref()
                .map_or(false, |e| e.active_debuffs.contains_key("bait"));
            if is_bait {
                action.multiplier = 3.0; // override to 300%
                state.stacks.insert(BAIT_HIT_FLAG.to_string(), 1.0);
            }
        }
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    let eidolon = state.team[idx].eidolon;
    if let Some(t) = target_idx {
        if action.action_type == ActionType::Skill
            && state.enemies[t].as_ref().map_or(false, |e| e.hp > 0.0)
        {
            // Apply / move Bait to this target
            apply_bait(state, t, eidolon);

            // SP refund if we hit existing Bait
            if state.stacks.get(BAIT_HIT_FLAG).copied().unwrap_or(0.0) >= 1.0 {
                state.stacks.insert(BAIT_HIT_FLAG.to_string(), 0.0);
                state.skill_points = (state.skill_points + 1).min(5);
            }

            // A2: skill → +1 Gluttony
            add_gluttony(state, eidolon, 1.0);
        }
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;

    // Apply Bait to first alive enemy
    let target = state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0));
    if let Some(t) = target {
        apply_bait(state, t, eidolon);
    } else {
        return;
    }
    let t = target.unwrap();

    // E4: +40% ATK for 3 turns
    if eidolon >= 4 {
        state.stacks.insert(E4_TURNS.to_string(), 3.0);
        state.team[idx].buffs.atk_percent += 40.0;
    }

    // Main hit: 400% ATK
    let mut ult_member = state.team[idx].clone();
    if eidolon >= 6 {
        ult_member.buffs.dmg_boost +=
            state.stacks.get(GLUTTONY_TOTAL).copied().unwrap_or(0.0) * 4.0;
    }
    let ult_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       4.0,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 30.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };
    let dmg = state.enemies[t].as_ref()
        .map(|e| damage::calculate_damage(&ult_member, e, &ult_action))
        .unwrap_or(0.0);
    if dmg > 0.0 {
        if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
    }
    if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[t] = None;
    }

    // Restore Charge to 3; A2: +2 Gluttony from ult
    state.stacks.insert(CHARGE_KEY.to_string(), 3.0);
    add_gluttony(state, eidolon, 2.0);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Ashveil Ult: {:.0} DMG", dmg));

    // Enhanced FUP chain
    launch_enhanced_fup(state, idx);
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {
    if get_charge(state) <= 0.0 { return; }

    let eidolon = state.team[idx].eidolon;
    let bait = match get_bait_idx(state)
        .or_else(|| move_bait_to_lowest(state, eidolon))
    {
        Some(t) => t,
        None    => return,
    };

    let killed = fire_fup(state, idx, bait, true);
    add_gluttony(state, eidolon, 2.0);
    if killed {
        add_gluttony(state, eidolon, 1.0);
        let _ = move_bait_to_lowest(state, eidolon);
    }
}
