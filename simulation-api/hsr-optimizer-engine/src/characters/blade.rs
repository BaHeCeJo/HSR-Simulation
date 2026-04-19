use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

const CHARGE_KEY:   &str = "blade_charge";
const HELLSCAPE_KEY:&str = "blade_hellscape";
const TALLY_KEY:    &str = "blade_tally";
const E4_KEY:       &str = "blade_e4";
const ENH_FLAG:     &str = "blade_enh";
const ENTERING_KEY: &str = "blade_entering";

fn max_charges(eidolon: i32) -> f64 {
    if eidolon >= 6 { 4.0 } else { 5.0 }
}

// ── Tally helpers ─────────────────────────────────────────────────────────────

fn add_to_tally(state: &mut SimState, idx: usize, amount: f64) {
    if amount <= 0.0 { return; }
    let cap = state.team[idx].max_hp * 0.90;
    let cur = state.stacks.get(TALLY_KEY).copied().unwrap_or(0.0);
    state.stacks.insert(TALLY_KEY.to_string(), (cur + amount).min(cap));
}

fn tally(state: &SimState) -> f64 {
    state.stacks.get(TALLY_KEY).copied().unwrap_or(0.0)
}

// ── HP consumption ────────────────────────────────────────────────────────────

fn consume_hp(state: &mut SimState, idx: usize, pct: f64) {
    let max_hp   = state.team[idx].max_hp;
    let loss     = (max_hp * pct).min((state.team[idx].hp - 1.0).max(0.0));
    if loss <= 0.0 { return; }
    state.team[idx].hp -= loss;
    add_to_tally(state, idx, loss);
    // E4 can stack twice; check after each HP event
    check_e4(state, idx);
    check_e4(state, idx);
    add_charge(state, idx);
}

// ── E4: Max HP boost when HP ≤ 50% ───────────────────────────────────────────

fn check_e4(state: &mut SimState, idx: usize) {
    if state.team[idx].eidolon < 4 { return; }
    let stacks = state.team[idx].stacks.get(E4_KEY).copied().unwrap_or(0.0) as u32;
    if stacks >= 2 { return; }
    if state.team[idx].hp / state.team[idx].max_hp <= 0.50 {
        state.team[idx].stacks.insert(E4_KEY.to_string(), (stacks + 1) as f64);
        // Permanently scale base HP and max_hp by +20%
        let old = state.team[idx].base_stats.get(ids::CHAR_HP_ID).copied().unwrap_or(0.0);
        state.team[idx].base_stats.insert(ids::CHAR_HP_ID.to_string(), old * 1.20);
        state.team[idx].max_hp *= 1.20;
    }
}

// ── Charge / Talent FUP ───────────────────────────────────────────────────────

fn add_charge(state: &mut SimState, idx: usize) {
    let max = max_charges(state.team[idx].eidolon);
    let cur = state.stacks.get(CHARGE_KEY).copied().unwrap_or(0.0);
    if cur >= max { return; }
    let next = cur + 1.0;
    state.stacks.insert(CHARGE_KEY.to_string(), next);
    if next >= max {
        fire_fup(state, idx);
    }
}

fn fire_fup(state: &mut SimState, idx: usize) {
    state.stacks.insert(CHARGE_KEY.to_string(), 0.0);

    let eidolon = state.team[idx].eidolon;
    // E6: +50% Max HP additive on top of 130% → 180%
    let fup_mult = if eidolon >= 6 { 1.80 } else { 1.30 };

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if alive.is_empty() { return; }

    // A6: +20% DMG on FUP
    let mut fup_member = state.team[idx].clone();
    fup_member.buffs.dmg_boost += 20.0;

    let fup_action = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
        multiplier:       fup_mult,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 10.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let mut total = 0.0f64;
    for &slot in &alive {
        if state.enemies[slot].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }
        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&fup_member, e, &fup_action))
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

    // Heal Blade: 25% Max HP; A4 boosts incoming healing by 20% → 30% effective
    let heal = state.team[idx].max_hp * 0.25 * 1.20;
    let max_hp = state.team[idx].max_hp;
    state.team[idx].hp = (state.team[idx].hp + heal).min(max_hp);
    // A4: 25% of the healed amount goes to tally
    add_to_tally(state, idx, heal * 0.25);

    // Talent: +10 energy; A6: +15 energy
    state.team[idx].energy =
        (state.team[idx].energy + 25.0).min(state.team[idx].max_energy);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Shuhu's Gift FUP: {:.0} DMG, healed {:.0}", total, heal));
}

// ── Enhanced Basic side-hits + E1 ────────────────────────────────────────────

fn enhanced_basic_extras(state: &mut SimState, idx: usize, main_slot: usize) {
    let eidolon = state.team[idx].eidolon;
    let t       = tally(state);
    let member  = state.team[idx].clone();

    // Adjacent: 52% Max HP
    let adj_action = ActionParams {
        action_type:      ActionType::Basic,
        scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
        multiplier:       0.52,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 10.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };
    let left  = if main_slot > 0 { Some(main_slot - 1) } else { None };
    let right = if main_slot + 1 < state.enemies.len() { Some(main_slot + 1) } else { None };

    let mut adj_total = 0.0;
    for adj in [left, right].iter().flatten() {
        if state.enemies[*adj].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }
        let dmg = state.enemies[*adj].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &adj_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[*adj].as_mut() { e.hp -= dmg; }
            adj_total += dmg;
        }
        if state.enemies[*adj].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[*adj] = None;
        }
    }
    state.total_damage += adj_total;

    // E1: extra hit to main target = 150% tally
    if eidolon >= 1 && t > 0.0 {
        let e1_action = ActionParams {
            action_type:      ActionType::Basic,
            scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
            multiplier:       0.0,
            extra_multiplier: 0.0,
            extra_dmg:        t * 1.50,
            toughness_damage: 0.0,
            inflicts_debuff:  false,
            is_ult_dmg:       false,
        };
        if state.enemies[main_slot].as_ref().map_or(false, |e| e.hp > 0.0) {
            let e1_dmg = state.enemies[main_slot].as_ref()
                .map(|e| damage::calculate_damage(&member, e, &e1_action))
                .unwrap_or(0.0);
            if e1_dmg > 0.0 {
                if let Some(e) = state.enemies[main_slot].as_mut() { e.hp -= e1_dmg; }
                state.total_damage += e1_dmg;
            }
            if state.enemies[main_slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[main_slot] = None;
            }
        }
    }
}

// ── Hooks ─────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy       = 130.0;
    state.team[idx].buffs.crit_rate  += 12.0; // minor trace: CRIT Rate +12%
    state.team[idx].buffs.hp_percent += 28.0; // minor trace: HP +28%
    state.team[idx].buffs.effect_res += 10.0; // minor trace: Effect RES +10%

    state.stacks.insert(CHARGE_KEY.to_string(),    0.0);
    state.stacks.insert(HELLSCAPE_KEY.to_string(), 0.0);
    state.stacks.insert(TALLY_KEY.to_string(),     0.0);
}

pub fn on_turn_start(_state: &mut SimState, _idx: usize) {}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    let hellscape = state.stacks.get(HELLSCAPE_KEY).copied().unwrap_or(0.0);

    if hellscape > 0.0 {
        // ── In Hellscape: every Basic/Skill becomes Enhanced Basic ──
        state.team[idx].buffs.dmg_boost += 40.0;
        if state.team[idx].eidolon >= 2 { state.team[idx].buffs.crit_rate += 15.0; }

        action.scaling_stat_id  = ids::CHAR_HP_ID.to_string();
        action.multiplier       = 1.30;
        action.toughness_damage = 20.0;
        action.extra_dmg        = 0.0;

        state.team[idx].stacks.insert(ENH_FLAG.to_string(), 1.0);
        consume_hp(state, idx, 0.10);
    } else if action.action_type == ActionType::Skill {
        // ── Enter Hellscape: Skill deals 0 damage, gives 0 energy ──
        action.multiplier       = 0.0;
        action.toughness_damage = 0.0;
        state.stacks.insert(HELLSCAPE_KEY.to_string(), 3.0);
        state.team[idx].stacks.insert(ENTERING_KEY.to_string(), 1.0);
        consume_hp(state, idx, 0.30);
        // Pre-apply Hellscape buffs so the inline Enhanced Basic (in on_after_action)
        // uses them while the snapshot is still active.
        state.team[idx].buffs.dmg_boost += 40.0;
        if state.team[idx].eidolon >= 2 { state.team[idx].buffs.crit_rate += 15.0; }
    } else {
        // ── Normal Basic ATK ──
        action.scaling_stat_id  = ids::CHAR_HP_ID.to_string();
        action.multiplier       = 0.50;
        action.toughness_damage = 10.0;
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    let is_enh      = state.team[idx].stacks.remove(ENH_FLAG).is_some();
    let is_entering = state.team[idx].stacks.remove(ENTERING_KEY).is_some();
    let err_mult    = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;

    if is_enh {
        // Fix SP / energy depending on the simulator's original action-type accounting
        match action.action_type {
            ActionType::Basic => {
                // Simulator added +1 SP and +20*err energy for Basic;
                // Enhanced Basic: 0 SP, +30*err energy → correct by +10*err energy and -1 SP
                state.skill_points = (state.skill_points - 1).max(0);
                state.team[idx].energy = (state.team[idx].energy + 10.0 * err_mult)
                    .min(state.team[idx].max_energy);
            }
            ActionType::Skill => {
                // Simulator deducted -1 SP and added +30*err energy;
                // Enhanced Basic in Hellscape: 0 SP, +30*err energy → refund 1 SP
                state.skill_points = (state.skill_points + 1).min(5);
            }
            _ => {}
        }

        // Decrement Hellscape counter
        let h = state.stacks.get(HELLSCAPE_KEY).copied().unwrap_or(0.0);
        if h > 0.0 { state.stacks.insert(HELLSCAPE_KEY.to_string(), h - 1.0); }

        // Adjacent hits + E1 (main hit was dealt by the simulator)
        if let Some(t) = target_idx {
            enhanced_basic_extras(state, idx, t);
        }
        // Charge from Enhanced Basic action
        add_charge(state, idx);
    }

    if is_entering {
        // Skill gave +30*err energy but should give 0 — undo it
        state.team[idx].energy = (state.team[idx].energy - 30.0 * err_mult).max(0.0);

        // "Skill does not end the current turn" → execute Enhanced Basic immediately
        let target_slot = state.enemies.iter()
            .position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0));

        if let Some(t) = target_slot {
            // HP cost for Enhanced Basic
            consume_hp(state, idx, 0.10);

            // Hellscape: 3 turns set, first used here → 2 remain after this
            state.stacks.insert(HELLSCAPE_KEY.to_string(), 2.0);

            // Main hit: 130% Max HP (simulator dealt 0 since multiplier was 0)
            let member = state.team[idx].clone();
            let main_action = ActionParams {
                action_type:      ActionType::Basic,
                scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
                multiplier:       1.30,
                extra_multiplier: 0.0,
                extra_dmg:        0.0,
                toughness_damage: 20.0,
                inflicts_debuff:  false,
                is_ult_dmg:       false,
            };
            if state.enemies[t].as_ref().map_or(false, |e| e.hp > 0.0) {
                let dmg = state.enemies[t].as_ref()
                    .map(|e| damage::calculate_damage(&member, e, &main_action))
                    .unwrap_or(0.0);
                if dmg > 0.0 {
                    if let Some(e) = state.enemies[t].as_mut() { e.hp -= dmg; }
                    state.total_damage += dmg;
                }
                if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
                    state.enemies[t] = None;
                }
            }

            // Adjacent + E1
            enhanced_basic_extras(state, idx, t);
        }

        // Energy from Enhanced Basic: +30
        state.team[idx].energy = (state.team[idx].energy + 30.0 * err_mult)
            .min(state.team[idx].max_energy);
        // Charge from this Enhanced Basic
        add_charge(state, idx);
    }

    // Ult: Talent grants +1 Charge when using Ultimate
    if action.action_type == ActionType::Ultimate {
        add_charge(state, idx);
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;

    // Set HP to 50% Max HP; HP sacrificed contributes to tally
    let target_hp = state.team[idx].max_hp * 0.50;
    let hp_loss   = (state.team[idx].hp - target_hp).max(0.0);
    state.team[idx].hp = target_hp.max(1.0);
    if hp_loss > 0.0 { add_to_tally(state, idx, hp_loss); }
    check_e4(state, idx);
    check_e4(state, idx);

    let t = tally(state);

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();

    if alive.is_empty() {
        // A2: reset tally to 50% Max HP
        let a2 = state.team[idx].max_hp * 0.50;
        state.stacks.insert(TALLY_KEY.to_string(), a2);
        return;
    }

    let main_t = alive[0];
    let member  = state.team[idx].clone();

    // Main target: 150% Max HP + 100% tally
    let main_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
        multiplier:       1.50,
        extra_multiplier: 0.0,
        extra_dmg:        t,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };

    let mut total = 0.0f64;
    if state.enemies[main_t].as_ref().map_or(false, |e| e.hp > 0.0) {
        let dmg = state.enemies[main_t].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &main_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[main_t].as_mut() { e.hp -= dmg; }
            total += dmg;
        }

        // E1: extra hit on main target = 150% tally
        if eidolon >= 1 && t > 0.0 {
            let e1_action = ActionParams {
                multiplier: 0.0, extra_dmg: t * 1.50, toughness_damage: 0.0,
                ..main_action.clone()
            };
            if state.enemies[main_t].as_ref().map_or(false, |e| e.hp > 0.0) {
                let e1 = state.enemies[main_t].as_ref()
                    .map(|e| damage::calculate_damage(&member, e, &e1_action))
                    .unwrap_or(0.0);
                if e1 > 0.0 {
                    if let Some(e) = state.enemies[main_t].as_mut() { e.hp -= e1; }
                    total += e1;
                }
            }
        }

        if state.enemies[main_t].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[main_t] = None;
        }
    }

    // Adjacent: 60% Max HP + 60% tally
    let adj_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
        multiplier:       0.60,
        extra_multiplier: 0.0,
        extra_dmg:        t * 0.60,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };
    let left  = if main_t > 0 { Some(main_t - 1) } else { None };
    let right = if main_t + 1 < state.enemies.len() { Some(main_t + 1) } else { None };
    for adj in [left, right].iter().flatten() {
        if state.enemies[*adj].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }
        let dmg = state.enemies[*adj].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &adj_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[*adj].as_mut() { e.hp -= dmg; }
            total += dmg;
        }
        if state.enemies[*adj].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[*adj] = None;
        }
    }

    state.total_damage += total;

    // A2: reset tally to 50% Max HP after ult
    let a2 = state.team[idx].max_hp * 0.50;
    state.stacks.insert(TALLY_KEY.to_string(), a2);

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Death Sentence: {:.0} DMG (tally was {:.0})", total, t));
}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_enemy_action(state: &mut SimState, idx: usize, _enemy_idx: usize) {
    // Talent: +1 Charge each time Blade is attacked (max 1 per attack event)
    add_charge(state, idx);
}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
