use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

// ─── Stack keys (TeamMember.stacks — not snapshotted) ─────────────────────────
const SUMMATION:       &str = "dr_summation";       // current Summation stacks (max 10)
const SUMMATION_BAKED: &str = "dr_summation_baked"; // stacks whose CR/CD are currently applied
const WF_TARGET:       &str = "dr_wf_target";       // enemy slot with Wiseman's Folly (-1 = none)
const WF_TRIGGERS:     &str = "dr_wf_triggers";     // remaining WF FUA triggers
const TALENT_ACC:      &str = "dr_talent_acc";       // threshold accumulator for Talent proc

// ─── Per-Summation-stack bonuses ──────────────────────────────────────────────
const CR_PER_STACK: f64 = 2.5;
const CD_PER_STACK: f64 = 5.0;

/// Compute the debuff count on the primary target (first alive enemy).
fn count_target_debuffs(state: &SimState) -> usize {
    let t = state.enemies.iter().find(|s| s.as_ref().map_or(false, |e| e.hp > 0.0));
    t.and_then(|s| s.as_ref()).map(|e| e.debuff_count as usize).unwrap_or(0)
}

fn first_alive_enemy(state: &SimState) -> Option<usize> {
    state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
}

/// Update Summation stacks from a new debuff count, cap at 10.
/// Returns new stack count.
fn update_summation(state: &mut SimState, idx: usize, dc: usize) -> f64 {
    let new_stacks = (dc as f64).min(10.0);
    state.team[idx].stacks.insert(SUMMATION, new_stacks);
    new_stacks
}

/// Remove old baked CR/CD from buffs and apply the new amount.
/// Called in on_turn_start (runs before snapshot) so the values persist through the turn.
fn reapply_summation_buffs(state: &mut SimState, idx: usize) {
    let old = state.team[idx].stacks.get(SUMMATION_BAKED).copied().unwrap_or(0.0);
    let new = state.team[idx].stacks.get(SUMMATION).copied().unwrap_or(0.0);

    // Remove old contribution
    state.team[idx].buffs.crit_rate -= old * CR_PER_STACK;
    state.team[idx].buffs.crit_dmg  -= old * CD_PER_STACK;

    // Apply new contribution
    state.team[idx].buffs.crit_rate += new * CR_PER_STACK;
    state.team[idx].buffs.crit_dmg  += new * CD_PER_STACK;

    state.team[idx].stacks.insert(SUMMATION_BAKED, new);
}

/// Fire the Talent FUA (270% ATK Imaginary Follow-Up).
/// A6 bonus is NOT re-added here — if called within on_after_action, A6 is already
/// present in the member clone (snapshot window). WF FUAs call this outside the window
/// and miss A6 — documented approximation.
fn fire_talent_fua(state: &mut SimState, idx: usize, target_slot: usize) {
    // Redirect to a living enemy if the original target died
    let t = if state.enemies[target_slot].as_ref().map_or(true, |e| e.hp <= 0.0) {
        match first_alive_enemy(state) {
            Some(i) => i,
            None => return,
        }
    } else {
        target_slot
    };

    let eidolon = state.team[idx].eidolon;
    let dc      = count_target_debuffs(state);

    // Build member clone; E6 adds +50% FUA DMG boost
    let mut member = state.team[idx].clone();
    if eidolon >= 6 {
        member.buffs.follow_up_dmg_boost += 50.0;
    }

    // Main 270% FUA hit
    let fua_action = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       2.70,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 10.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let dmg = state.enemies[t].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &fua_action))
        .unwrap_or(0.0);
    if dmg > 0.0 {
        if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
        if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[t] = None;
        }
    }

    // E4: +15 energy on FUA
    let energy_gain = if eidolon >= 4 { 20.0 } else { 5.0 };
    state.team[idx].energy = (state.team[idx].energy + energy_gain).min(state.team[idx].max_energy);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Talent FUA: {:.0} DMG", dmg));

    // E2: additional hits equal to min(dc, 4), each 20% ATK
    if eidolon >= 2 && dc > 0 {
        let extra_hits = dc.min(4);
        let extra_action = ActionParams {
            action_type:      ActionType::FollowUp,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       0.20,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 0.0,
            inflicts_debuff:  false,
            is_ult_dmg:       false,
        };
        let mut extra_total = 0.0;
        for _ in 0..extra_hits {
            // Redirect if main target died
            let et = if state.enemies[t].as_ref().map_or(true, |e| e.hp <= 0.0) {
                match first_alive_enemy(state) { Some(i) => i, None => break }
            } else { t };

            let d = state.enemies[et].as_ref()
                .map(|e| damage::calculate_damage(&member, e, &extra_action))
                .unwrap_or(0.0);
            if d > 0.0 {
                if let Some(e) = state.enemies[et].as_mut() { e.hp -= d; }
                state.total_damage += d;
                extra_total += d;
                if state.enemies[et].as_ref().map_or(false, |e| e.hp <= 0.0) {
                    state.enemies[et] = None;
                }
            }
        }
        if extra_total > 0.0 {
            state.add_log(&name, format!("E2 extra hits ×{}: {:.0} DMG", extra_hits, extra_total));
        }
    }
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 140.0;
    state.team[idx].is_fua     = true;

    // Minor traces
    state.team[idx].buffs.crit_rate  += 10.0; // A2 minor trace: CR +10%
    state.team[idx].buffs.dmg_boost  += 24.0; // minor trace: Imaginary DMG +24%
    state.team[idx].buffs.effect_hit_rate += 18.0; // minor trace: EHR +18%

    state.team[idx].stacks.insert(SUMMATION,       0.0);
    state.team[idx].stacks.insert(SUMMATION_BAKED, 0.0);
    state.team[idx].stacks.insert(WF_TARGET,       -1.0);
    state.team[idx].stacks.insert(WF_TRIGGERS,      0.0);
    state.team[idx].stacks.insert(TALENT_ACC,       0.0);

    // E1: start with 4 Summation stacks
    if state.team[idx].eidolon >= 1 {
        state.team[idx].stacks.insert(SUMMATION, 4.0);
        state.team[idx].buffs.crit_rate += 4.0 * CR_PER_STACK;
        state.team[idx].buffs.crit_dmg  += 4.0 * CD_PER_STACK;
        state.team[idx].stacks.insert(SUMMATION_BAKED, 4.0);
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // Reapply Summation stats (stacks may have changed since last turn via on_after_action)
    reapply_summation_buffs(state, idx);
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    if action.action_type != ActionType::Skill { return; }

    let dc = count_target_debuffs(state);
    let max_stacks = if state.team[idx].eidolon >= 1 { 10usize } else { 5usize };
    let effective = dc.min(max_stacks);

    // A2: Skill gains +CR and +CD based on debuffs (auto-reverted after action)
    state.team[idx].buffs.crit_rate += effective as f64 * CR_PER_STACK;
    state.team[idx].buffs.crit_dmg  += effective as f64 * CD_PER_STACK;

    // A6: if 3+ debuffs, add up to +50% DMG boost (10% per debuff, capped at 5)
    if dc >= 3 {
        let boost = (dc.min(5) as f64) * 10.0;
        state.team[idx].buffs.dmg_boost += boost;
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Skill pre-action: +{:.0}% CR, +{:.0}% CD ({}debuffs), A6 +{:.0}% DMG",
        effective as f64 * CR_PER_STACK,
        effective as f64 * CD_PER_STACK,
        dc,
        if dc >= 3 { (dc.min(5) as f64) * 10.0 } else { 0.0 },
    ));
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    if action.action_type != ActionType::Skill { return; }

    let t = match target_idx.or_else(|| first_alive_enemy(state)) {
        Some(i) => i,
        None => return,
    };

    let eidolon = state.team[idx].eidolon;

    // A4: apply -10% Effect RES debuff from Skill (lasts 2 turns)
    // Applied before counting so it is included in the debuff tally below.
    if let Some(enemy) = state.enemies[t].as_mut() {
        effects::apply_enemy_debuff(enemy, "dr_ratio_a4_eres", StatusEffect {
            duration: 2,
            value:    10.0,
            stat:     Some("effect_res".to_string()),
            effects:  vec![],
        });
    }

    // Count debuffs on target (includes A4 just applied)
    let dc = state.enemies[t].as_ref().map(|e| e.debuff_count as usize).unwrap_or(0);

    // Update Summation stacks (capped at 10; 0 stacks requires 0 debuffs — never drops)
    let new_stacks = update_summation(state, idx, dc);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Skill: A4 –10% ERs on enemy, {} debuffs → {:.0} Summation stacks",
        dc, new_stacks,
    ));

    // Talent: 40% per Skill (threshold accumulator)
    let acc = state.team[idx].stacks.get(TALENT_ACC).copied().unwrap_or(0.0) + 0.40;
    if acc >= 1.0 {
        state.team[idx].stacks.insert(TALENT_ACC, acc - 1.0);
        fire_talent_fua(state, idx, t);
    } else {
        state.team[idx].stacks.insert(TALENT_ACC, acc);
    }

    // E6: after Skill, immediately fire an additional Talent FUA
    if eidolon >= 6 {
        fire_talent_fua(state, idx, t);
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);

    let eidolon = state.team[idx].eidolon;
    state.team[idx].energy = 5.0;

    // Mark the first alive enemy as Wiseman's Folly target
    let target = first_alive_enemy(state);
    match target {
        Some(t) => {
            state.team[idx].stacks.insert(WF_TARGET, t as f64);
            let triggers = if eidolon >= 6 { 3.0 } else { 2.0 };
            state.team[idx].stacks.insert(WF_TRIGGERS, triggers);
            let name = state.team[idx].name.clone();
            state.add_log(&name, format!(
                "Ult: Wiseman's Folly on enemy[{}] ({:.0} triggers)",
                t, triggers,
            ));
        }
        None => {
            state.team[idx].stacks.insert(WF_TARGET,   -1.0);
            state.team[idx].stacks.insert(WF_TRIGGERS,  0.0);
        }
    }
}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,
    _source_idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // Only trigger on attack actions that hit the WF target
    if !matches!(action.action_type, ActionType::Basic | ActionType::Skill | ActionType::Ultimate | ActionType::FollowUp) {
        return;
    }

    let wf_target  = state.team[idx].stacks.get(WF_TARGET).copied().unwrap_or(-1.0);
    let wf_triggers = state.team[idx].stacks.get(WF_TRIGGERS).copied().unwrap_or(0.0);

    if wf_triggers <= 0.0 || wf_target < 0.0 { return; }

    let wf_slot = wf_target as usize;
    let hit_wf = target_idx.map_or(false, |t| t == wf_slot);
    if !hit_wf { return; }

    // Consume one trigger, fire FUA
    state.team[idx].stacks.insert(WF_TRIGGERS, wf_triggers - 1.0);
    if wf_triggers - 1.0 <= 0.0 {
        state.team[idx].stacks.insert(WF_TARGET, -1.0);
    }

    fire_talent_fua(state, idx, wf_slot);
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
