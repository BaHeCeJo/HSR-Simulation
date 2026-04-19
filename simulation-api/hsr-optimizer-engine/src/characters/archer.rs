use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

const ENERGY_KEY: &str   = "archer_energy";
const CHARGE_KEY: &str   = "archer_charges";
const CC_KEY: &str       = "archer_cc";       // 1 = Circuit Connection active
const CC_USES_KEY: &str  = "archer_cc_uses";  // skill uses in current CC rotation
const CC_DMG_KEY: &str   = "archer_cc_dmg";   // CC DMG boost stacks
const E1_DONE_KEY: &str  = "archer_e1_done";  // E1 SP refund triggered this rotation
const ENERGY_CAP: f64    = 220.0;
const MAX_CC_USES: usize = 5;

// ── Energy helpers ────────────────────────────────────────────────────────────

fn add_energy(state: &mut SimState, idx: usize, amount: f64) {
    let cur = state.stacks.get(ENERGY_KEY).copied().unwrap_or(0.0);
    state.stacks.insert(ENERGY_KEY.to_string(), (cur + amount).min(ENERGY_CAP));
    if state.stacks.get(ENERGY_KEY).copied().unwrap_or(0.0) >= ENERGY_CAP {
        state.team[idx].stacks.insert("_ult_ready".to_string(), 1.0);
    }
}

// ── Charge helpers ────────────────────────────────────────────────────────────

fn get_charges(state: &SimState) -> f64 {
    state.stacks.get(CHARGE_KEY).copied().unwrap_or(0.0)
}

fn add_charge(state: &mut SimState, n: f64) {
    state.stacks.insert(CHARGE_KEY.to_string(), (get_charges(state) + n).min(4.0));
}

// ── CC state helpers ──────────────────────────────────────────────────────────

fn in_cc(state: &SimState) -> bool {
    state.stacks.get(CC_KEY).copied().unwrap_or(0.0) >= 1.0
}

fn get_cc_uses(state: &SimState) -> usize {
    state.stacks.get(CC_USES_KEY).copied().unwrap_or(0.0) as usize
}

fn get_cc_dmg(state: &SimState) -> f64 {
    state.stacks.get(CC_DMG_KEY).copied().unwrap_or(0.0)
}

fn max_cc_dmg_stacks(eidolon: i32) -> f64 {
    if eidolon >= 6 { 3.0 } else { 2.0 }
}

fn enter_cc(state: &mut SimState) {
    state.stacks.insert(CC_KEY.to_string(), 1.0);
    state.stacks.insert(CC_USES_KEY.to_string(), 1.0);
    state.stacks.insert(CC_DMG_KEY.to_string(), 0.0);
    state.stacks.insert(E1_DONE_KEY.to_string(), 0.0);
}

fn exit_cc(state: &mut SimState) {
    state.stacks.insert(CC_KEY.to_string(), 0.0);
    state.stacks.insert(CC_USES_KEY.to_string(), 0.0);
    state.stacks.insert(CC_DMG_KEY.to_string(), 0.0);
    state.stacks.insert(E1_DONE_KEY.to_string(), 0.0);
}

/// Execute one CC Skill hit: 360% ATK × (1 + cc_dmg_stacks * 100% DMG boost).
fn execute_cc_skill(state: &mut SimState, idx: usize) {
    let cc_dmg = get_cc_dmg(state);
    let eidolon = state.team[idx].eidolon;
    let mut cc_member = state.team[idx].clone();
    cc_member.buffs.dmg_boost += cc_dmg * 100.0;
    if eidolon >= 6 {
        cc_member.buffs.def_ignore += 20.0;
    }
    // Apply A6 CRIT DMG buff if active
    let a6_val = state.team[idx].active_buffs.get("archer_a6").map(|b| b.value).unwrap_or(0.0);
    if a6_val > 0.0 { cc_member.buffs.crit_dmg += a6_val; }

    let cc_action = ActionParams {
        action_type:      ActionType::Skill,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       3.60,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let target = state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0));
    if let Some(t) = target {
        let dmg = state.enemies[t].as_ref()
            .map(|e| damage::calculate_damage(&cc_member, e, &cc_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
            state.total_damage += dmg;
        }
        if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[t] = None;
        }
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("[CC Skill {}] {:.0} DMG (cc_stacks={})",
            get_cc_uses(state), dmg, cc_dmg as i32));
    }
}

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy      = f64::MAX; // custom energy
    state.team[idx].buffs.dmg_boost   += 22.4; // minor trace: Quantum DMG
    state.team[idx].buffs.atk_percent += 18.0; // minor trace
    state.team[idx].buffs.crit_rate   +=  6.7; // minor trace

    state.stacks.insert(ENERGY_KEY.to_string(), 0.0);
    state.stacks.insert(CHARGE_KEY.to_string(), 0.0);
    state.stacks.insert(CC_KEY.to_string(), 0.0);

    // A4: start with 1 Charge
    add_charge(state, 1.0);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // E6: recover 1 SP for allies at turn start
    if state.team[idx].eidolon >= 6 {
        state.skill_points = (state.skill_points + 1).min(7);
        // A6: if SP ≥ 4, grant +120% CRIT DMG for 1 turn
        if state.skill_points >= 4 {
            use crate::models::StatusEffect;
            use crate::effects;
            effects::apply_member_buff(&mut state.team[idx], "archer_a6", StatusEffect {
                duration: 1, value: 120.0, stat: None, effects: vec![],
            });
        }
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    // A6 CRIT DMG buff
    let a6_val = state.team[idx].active_buffs.get("archer_a6").map(|b| b.value).unwrap_or(0.0);
    if a6_val > 0.0 {
        state.team[idx].buffs.crit_dmg += a6_val;
    }

    // E4: Ult DMG +150%
    if action.action_type == ActionType::Ultimate && state.team[idx].eidolon >= 4 {
        state.team[idx].buffs.dmg_boost += 150.0;
    }

    // E6: Skill DEF ignore +20%
    if action.action_type == ActionType::Skill && state.team[idx].eidolon >= 6 {
        state.team[idx].buffs.def_ignore += 20.0;
    }

    // CC DMG boost for first/normal skill hit
    if action.action_type == ActionType::Skill && in_cc(state) {
        state.team[idx].buffs.dmg_boost += get_cc_dmg(state) * 100.0;
    }

    // Prevent simulator from adding energy (managed manually)
    state.team[idx].energy = 0.0;
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    _target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            add_energy(state, idx, 20.0);
            // A6: check SP threshold for next turn
            if state.skill_points >= 4 {
                use crate::models::StatusEffect;
                use crate::effects;
                effects::apply_member_buff(&mut state.team[idx], "archer_a6", StatusEffect {
                    duration: 1, value: 120.0, stat: None, effects: vec![],
                });
            }
            state.team[idx].energy = 0.0;
        }
        ActionType::Skill => {
            let eidolon = state.team[idx].eidolon;

            if !in_cc(state) {
                // First skill use: enter CC (use #1, dmg_stacks = 0)
                enter_cc(state);
            } else {
                // Subsequent CC Skill: increment DMG stack
                let new_dmg = (get_cc_dmg(state) + 1.0).min(max_cc_dmg_stacks(eidolon));
                state.stacks.insert(CC_DMG_KEY.to_string(), new_dmg);
                let new_uses = get_cc_uses(state) + 1;
                state.stacks.insert(CC_USES_KEY.to_string(), new_uses as f64);
            }

            add_energy(state, idx, 30.0);

            // E1: after 3 uses in CC rotation → recover 2 SP (once per rotation)
            let uses = get_cc_uses(state);
            if eidolon >= 1 && uses >= 3
                && state.stacks.get(E1_DONE_KEY).copied().unwrap_or(0.0) < 1.0
            {
                state.stacks.insert(E1_DONE_KEY.to_string(), 1.0);
                state.skill_points = (state.skill_points + 2).min(7);
                if state.skill_points >= 4 {
                    use crate::models::StatusEffect;
                    use crate::effects;
                    effects::apply_member_buff(&mut state.team[idx], "archer_a6", StatusEffect {
                        duration: 1, value: 120.0, stat: None, effects: vec![],
                    });
                }
            }

            // Execute additional CC Skill hits inline (simulating extra turns)
            // Continue as long as within 5-use cap and SP is available
            while get_cc_uses(state) < MAX_CC_USES && state.skill_points > 0 {
                state.skill_points -= 1;

                // Increment CC DMG stack before next hit
                let new_dmg = (get_cc_dmg(state) + 1.0).min(max_cc_dmg_stacks(eidolon));
                state.stacks.insert(CC_DMG_KEY.to_string(), new_dmg);
                let new_uses = get_cc_uses(state) + 1;
                state.stacks.insert(CC_USES_KEY.to_string(), new_uses as f64);

                execute_cc_skill(state, idx);
                add_energy(state, idx, 30.0);

                // E1: SP refund after 3rd use
                let uses2 = get_cc_uses(state);
                if eidolon >= 1 && uses2 >= 3
                    && state.stacks.get(E1_DONE_KEY).copied().unwrap_or(0.0) < 1.0
                {
                    state.stacks.insert(E1_DONE_KEY.to_string(), 1.0);
                    state.skill_points = (state.skill_points + 2).min(7);
                }
            }

            // Exit CC after chain completes
            exit_cc(state);
            state.team[idx].energy = 0.0;
        }
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].stacks.remove("_ult_ready");
    state.stacks.insert(ENERGY_KEY.to_string(), 5.0);
    state.team[idx].energy = 0.0;

    let eidolon = state.team[idx].eidolon;

    // Exit CC if active
    if in_cc(state) { exit_cc(state); }

    let target = state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0));
    if let Some(t) = target {
        let ult_member = state.team[idx].clone();

        // E4: already applied in on_before_action (dmg_boost += 150)
        // E2: -20% Quantum RES + Quantum Weakness for 2 turns
        if eidolon >= 2 {
            if let Some(e) = state.enemies[t].as_mut() {
                let prev_q = e.elemental_res.get("Quantum").copied().unwrap_or(e.resistance);
                e.elemental_res.insert("Quantum".to_string(), (prev_q - 0.20).max(-1.0));
                if !e.weaknesses.contains(&"Quantum".to_string()) {
                    e.weaknesses.push("Quantum".to_string());
                }
                e.active_debuffs.insert("archer_e2_qres".to_string(), crate::models::StatusEffect {
                    duration: 2, value: 20.0,
                    stat: Some("Quantum RES".to_string()), effects: vec![],
                });
            }
        }

        let ult_action = ActionParams {
            action_type:      ActionType::Ultimate,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       10.0,
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

        // Gain 2 Charges
        add_charge(state, 2.0);

        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Unlimited Blade Works: {:.0} DMG", dmg));
    }
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
    source_idx: usize,
    _action: &ActionParams,
    target_idx: Option<usize>,
) {
    // Talent: fire FUP after any teammate action if Charges > 0
    if source_idx == idx { return; } // not self
    if get_charges(state) <= 0.0 { return; }

    let target = target_idx.or_else(|| {
        state.enemies.iter().position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
    });
    let t = match target {
        Some(t) if state.enemies[t].as_ref().map_or(false, |e| e.hp > 0.0) => t,
        _ => return,
    };

    // Consume 1 Charge
    state.stacks.insert(CHARGE_KEY.to_string(), get_charges(state) - 1.0);

    // Relic: reset Ashblazing stacks for new FUA; will increment once after hit.
    crate::relics::on_follow_up_start(&mut state.team, idx);

    let eidolon = state.team[idx].eidolon;
    let mut fup_member = state.team[idx].clone();
    let a6_val = fup_member.active_buffs.get("archer_a6").map(|b| b.value).unwrap_or(0.0);
    if a6_val > 0.0 { fup_member.buffs.crit_dmg += a6_val; }

    let fup_action = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       2.0,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 10.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };
    let dmg = state.enemies[t].as_ref()
        .map(|e| damage::calculate_damage(&fup_member, e, &fup_action))
        .unwrap_or(0.0);
    if dmg > 0.0 {
        if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
    }
    if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[t] = None;
    }

    // Relic: 1 hit per FUP → increment Ashblazing stack; set Wind-Soaring window.
    crate::relics::on_follow_up_hit(&mut state.team, idx);
    crate::relics::on_follow_up_end(&mut state.team, idx);

    // Recover 1 SP from FUP
    state.skill_points = (state.skill_points + 1).min(7);
    // +5 energy per FUP
    add_energy(state, idx, 5.0);

    // A6: check SP threshold
    if state.skill_points >= 4 {
        use crate::models::StatusEffect;
        use crate::effects;
        effects::apply_member_buff(&mut state.team[idx], "archer_a6", StatusEffect {
            duration: 1, value: 120.0, stat: None, effects: vec![],
        });
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Archer Talent FUP: {:.0} DMG (Charge={:.0})",
        dmg, get_charges(state)));
    let _ = eidolon;
}
