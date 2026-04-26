use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, ActorEntry, SimState, StatusEffect};

// ─── Helpers ──────────────────────────────────────────────────────────────────

pub fn is_nw_alive(state: &SimState) -> bool {
    state.stacks.get("netherwing_alive").copied().unwrap_or(0.0) >= 1.0
}

fn add_newbud(state: &mut SimState, idx: usize, amount: f64) {
    if is_nw_alive(state) { return; }
    let cur = state.stacks.get("castorice_newbud").copied().unwrap_or(0.0);
    let max = state.stacks.get("castorice_newbud_max").copied().unwrap_or(9600.0);
    let new = (cur + amount).min(max);
    state.stacks.insert("castorice_newbud".to_string(), new);
    if new >= max {
        state.team[idx].stacks.insert("_ult_ready", 1.0);
    }
}

fn add_nw_hp(state: &mut SimState, amount: f64) {
    let cur     = state.stacks.get("netherwing_hp").copied().unwrap_or(0.0);
    let max_hp  = state.stacks.get("netherwing_max_hp").copied().unwrap_or(0.0);
    let new     = (cur + amount).min(max_hp);
    state.stacks.insert("netherwing_hp".to_string(), new);
}

/// Gain 1 Talent DMG stack (max 3, each lasts 3 Castorice turns).
fn add_dmg_stack(state: &mut SimState, idx: usize) {
    let stacks = state.stacks.get("castorice_dmg_stacks").copied().unwrap_or(0.0) as usize;
    if stacks < 3 {
        state.stacks.insert(format!("castorice_dmg_rem_{stacks}"), 3.0);
        state.stacks.insert("castorice_dmg_stacks".to_string(), (stacks + 1) as f64);
        state.team[idx].buffs.dmg_boost += 20.0;
    } else {
        // Refresh the stack with the least remaining turns
        for i in 0..3usize {
            let rem = state.stacks.get(&format!("castorice_dmg_rem_{i}")).copied().unwrap_or(3.0);
            if rem < 3.0 {
                state.stacks.insert(format!("castorice_dmg_rem_{i}"), 3.0);
                break;
            }
        }
    }
}

/// Decrement all active DMG stacks by 1 Castorice turn.
fn tick_dmg_stacks(state: &mut SimState, idx: usize) {
    let mut active = 0usize;
    for i in 0..3usize {
        let key = format!("castorice_dmg_rem_{i}");
        let rem = state.stacks.get(&key).copied().unwrap_or(0.0);
        if rem > 0.0 {
            let new = rem - 1.0;
            if new <= 0.0 {
                state.stacks.remove(&key);
            } else {
                state.stacks.insert(key, new);
                active += 1;
            }
        }
    }
    let old = state.stacks.get("castorice_dmg_stacks").copied().unwrap_or(0.0) as usize;
    let diff = active as i64 - old as i64;
    if diff != 0 {
        state.stacks.insert("castorice_dmg_stacks".to_string(), active as f64);
        state.team[idx].buffs.dmg_boost += diff as f64 * 20.0;
    }
}

/// Apply or remove A4 SPD +40% based on current HP ratio.
fn update_a4_spd(state: &mut SimState, idx: usize) {
    let hp_ratio = if state.team[idx].max_hp > 0.0 {
        state.team[idx].hp / state.team[idx].max_hp
    } else { 0.0 };
    let active = state.stacks.get("castorice_a4_active").copied().unwrap_or(0.0) >= 1.0;
    let should = hp_ratio >= 0.5;
    match (active, should) {
        (false, true) => {
            let spd = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
            state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), spd * 1.4);
            state.stacks.insert("castorice_a4_active".to_string(), 1.0);
        }
        (true, false) => {
            let spd = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
            state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), spd / 1.4);
            state.stacks.insert("castorice_a4_active".to_string(), 0.0);
        }
        _ => {}
    }
}

/// Consume `pct` of each alive team member's current HP (floored at 1).
/// Returns total HP consumed.
fn consume_ally_hp(state: &mut SimState, pct: f64) -> f64 {
    let mut total = 0.0;
    for i in 0..state.team.len() {
        if state.team[i].is_downed { continue; }
        let hp      = state.team[i].hp;
        let consume = (hp * pct).min(hp - 1.0).max(0.0);
        state.team[i].hp -= consume;
        total += consume;
    }
    total
}

/// AoE hit using Castorice's stats and HP-scaled `extra_dmg`.
/// `e1_mult`: E1 conditional multiplier (1.0 if not E1, checked per enemy slot).
fn aoe_hp_hit(state: &mut SimState, idx: usize, extra_dmg: f64, toughness: f64, action_type: ActionType) -> f64 {
    let eidolon = state.team[idx].eidolon;
    let member  = state.team[idx].clone();
    let action  = ActionParams {
        action_type,
        scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
        multiplier:       0.0,
        extra_multiplier: 0.0,
        extra_dmg,
        toughness_damage: toughness,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };
    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    let mut total = 0.0;
    for &slot in &alive {
        let e1_mult = if eidolon >= 1 {
            let e_hp      = state.enemies[slot].as_ref().map(|e| e.hp).unwrap_or(0.0);
            let casto_max = member.max_hp;
            if e_hp <= casto_max * 0.50 { 1.40 }
            else if e_hp <= casto_max * 0.80 { 1.20 }
            else { 1.0 }
        } else { 1.0 };

        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &action) * e1_mult)
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
    total
}

/// Netherwing Claw Splits the Veil: AoE 40% Castorice Max HP.
fn nw_claw(state: &mut SimState, idx: usize) -> f64 {
    let extra = state.team[idx].max_hp * 0.40;
    aoe_hp_hit(state, idx, extra, 10.0, ActionType::TalentProc)
}

/// One use of Breath Scorches the Shadow.
/// `seq`: 0=24%, 1=28%, 2+=34%. `a6`: stacks of A6 already accumulated.
fn nw_breath_hit(state: &mut SimState, idx: usize, seq: i32, a6: f64) -> f64 {
    let pct = match seq { 0 => 0.24, 1 => 0.28, _ => 0.34 };
    let extra = state.team[idx].max_hp * pct;

    let eidolon = state.team[idx].eidolon;
    // A6: +30% DMG per Breath use (stacks up to 6, lasts until end of this turn)
    // E6: +20% Quantum RES PEN for Netherwing
    // Apply temporarily before calculating
    state.team[idx].buffs.dmg_boost += a6 * 30.0;
    if eidolon >= 6 { state.team[idx].buffs.res_pen += 20.0; }

    let dmg = aoe_hp_hit(state, idx, extra, 10.0, ActionType::TalentProc);

    state.team[idx].buffs.dmg_boost -= a6 * 30.0;
    if eidolon >= 6 { state.team[idx].buffs.res_pen -= 20.0; }
    dmg
}

/// Wings Sweep the Ruins: bounce hits + team heal.
fn nw_wings_sweep(state: &mut SimState, idx: usize) -> f64 {
    let eidolon = state.team[idx].eidolon;
    let bounces = if eidolon >= 6 { 9 } else { 6 };
    let member  = state.team[idx].clone();
    let extra   = member.max_hp * 0.40;
    let action  = ActionParams {
        action_type:      ActionType::TalentProc,
        scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
        multiplier:       0.0,
        extra_multiplier: 0.0,
        extra_dmg:        extra,
        toughness_damage: 5.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };
    let mut total = 0.0;
    for k in 0..bounces {
        let alive: Vec<usize> = state.enemies.iter().enumerate()
            .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
            .collect();
        if alive.is_empty() { break; }
        let pick = alive[k % alive.len()];
        let e1_mult = if eidolon >= 1 {
            let e_hp      = state.enemies[pick].as_ref().map(|e| e.hp).unwrap_or(0.0);
            let casto_max = member.max_hp;
            if e_hp <= casto_max * 0.50 { 1.40 }
            else if e_hp <= casto_max * 0.80 { 1.20 }
            else { 1.0 }
        } else { 1.0 };
        let dmg = state.enemies[pick].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &action) * e1_mult)
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[pick].as_mut() { e.hp -= dmg; }
            total += dmg;
        }
        if state.enemies[pick].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[pick] = None;
        }
    }
    state.total_damage += total;

    // Restore 6% Max HP + 800 to all allies
    let heal = member.max_hp * 0.06 + 800.0;
    for i in 0..state.team.len() {
        if !state.team[i].is_downed {
            state.team[i].hp = (state.team[i].hp + heal).min(state.team[i].max_hp);
        }
    }
    total
}

fn dismiss_netherwing(state: &mut SimState, idx: usize) {
    state.stacks.insert("netherwing_alive".to_string(), 0.0);
    // Dispel Territory
    for slot in state.enemies.iter_mut() {
        if let Some(e) = slot.as_mut() {
            if e.active_debuffs.remove("castorice_territory").is_some() {
                e.debuff_count = e.debuff_count.saturating_sub(1);
                effects::recompute_enemy_caches(e);
            }
        }
    }
    let name = state.team[idx].name.clone();
    state.add_log(&name, "Netherwing disappears — Lost Netherland dispelled".to_string());
}

// ─── Netherwing turn (called directly by simulator.rs) ───────────────────────

pub fn netherwing_turn(state: &mut SimState, idx: usize) {
    if !is_nw_alive(state) { return; }

    let countdown     = state.stacks.get("netherwing_countdown").copied().unwrap_or(0.0);
    let new_countdown = (countdown - 1.0).max(0.0);
    state.stacks.insert("netherwing_countdown".to_string(), new_countdown);

    let claw_used = state.stacks.get("castorice_nw_claw_used").copied().unwrap_or(0.0) >= 1.0;
    let name      = state.team[idx].name.clone();

    if !claw_used {
        // Turn 1: Claw — preserve HP for Castorice's Boneclaw on her next turn
        let dmg = nw_claw(state, idx);
        state.stacks.insert("castorice_nw_claw_used".to_string(), 1.0);
        state.add_log(&name, format!("Netherwing Claw: {:.0} AoE DMG", dmg));

        if new_countdown <= 0.0 {
            let ws = nw_wings_sweep(state, idx);
            dismiss_netherwing(state, idx);
            state.add_log(&name, format!("Wings Sweep (timeout): {:.0} DMG", ws));
        }
        return;
    }

    // Subsequent turns: full Breath chain → Wings Sweep
    let eidolon  = state.team[idx].eidolon;
    let nw_max   = state.stacks.get("netherwing_max_hp").copied().unwrap_or(9600.0);
    let mut nw_hp    = state.stacks.get("netherwing_hp").copied().unwrap_or(nw_max);
    let mut seq      = state.stacks.get("netherwing_breath_seq").copied().unwrap_or(0.0) as i32;
    let mut ardent   = if eidolon >= 2 { state.stacks.get("castorice_e2_ardent").copied().unwrap_or(0.0) } else { 0.0 };
    let mut a6       = 0.0f64;
    let mut total    = 0.0f64;
    let mut n_hits   = 0usize;
    let castorice_kit = state.team[idx].kit_id.clone();

    loop {
        if nw_hp <= nw_max * 0.25 {
            // HP at threshold → Wings Sweep instead of Breath
            let ws = nw_wings_sweep(state, idx);
            total += ws;
            dismiss_netherwing(state, idx);
            state.add_log(&name, format!(
                "Wings Sweep ({n_hits} breaths): {:.0} DMG (WS: {:.0})",
                total - ws, ws,
            ));
            break;
        }

        // Consume 25% max HP (or use Ardent Will to offset)
        if ardent >= 1.0 {
            ardent -= 1.0;
            // E2: advance Castorice by 100% when Ardent Will is used
            let current_av = state.current_av;
            let old: Vec<ActorEntry> = state.av_queue.drain().collect();
            let mut skipped = false;
            for e in old {
                if !e.is_enemy && e.actor_id == castorice_kit && !skipped {
                    skipped = true;
                } else {
                    state.av_queue.push(e);
                }
            }
            state.av_queue.push(ActorEntry {
                next_av:     current_av + 0.01,
                actor_id:    castorice_kit.clone(),
                instance_id: castorice_kit.clone(),
                is_enemy:    false,
            });
        } else {
            nw_hp -= nw_max * 0.25;
            if nw_hp < 0.0 { nw_hp = 0.0; }
            state.stacks.insert("netherwing_hp".to_string(), nw_hp);
        }

        a6 = (a6 + 1.0).min(6.0);
        let dmg = nw_breath_hit(state, idx, seq, a6);
        total  += dmg;
        n_hits += 1;
        seq     = (seq + 1).min(2);
    }

    state.stacks.insert("netherwing_breath_seq".to_string(), seq as f64);
    state.stacks.insert("castorice_e2_ardent".to_string(), ardent);
    state.add_log(&name, format!("Netherwing Breath chain total: {:.0} DMG", total));

    // Tick Roar Rumbles
    tick_roar_rembles(state, idx);
}

fn tick_roar_rembles(state: &mut SimState, idx: usize) {
    let rem = state.stacks.get("castorice_roar_rem").copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }
    let new = rem - 1.0;
    if new <= 0.0 {
        for i in 0..state.team.len() {
            if !state.team[i].is_downed { state.team[i].buffs.dmg_boost -= 10.0; }
        }
        state.stacks.remove("castorice_roar_rem");
    } else {
        state.stacks.insert("castorice_roar_rem".to_string(), new);
    }
    let _ = idx;
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = f64::MAX; // custom Newbud system

    // Minor traces
    state.team[idx].buffs.crit_rate += 18.7;
    state.team[idx].buffs.crit_dmg  += 13.3;
    state.team[idx].buffs.dmg_boost += 14.4; // Quantum DMG +14.4%

    // Max Newbud: sum(level × 30) for all team members
    let max_newbud: f64 = state.team.iter().map(|m| m.level as f64 * 30.0).sum();
    state.stacks.insert("castorice_newbud".to_string(),     0.0);
    state.stacks.insert("castorice_newbud_max".to_string(), max_newbud);
    state.stacks.insert("netherwing_alive".to_string(),     0.0);
    state.stacks.insert("netherwing_gen".to_string(),       0.0);
    state.stacks.insert("castorice_dmg_stacks".to_string(), 0.0);
    state.stacks.insert("castorice_a4_active".to_string(),  0.0);
    state.stacks.insert("castorice_e2_ardent".to_string(),  0.0);

    // A4: apply SPD boost if starting HP ≥ 50% (true at full HP = 100%)
    update_a4_spd(state, idx);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // Suppress normal energy accumulation
    state.team[idx].energy = 0.0;

    tick_dmg_stacks(state, idx);
    update_a4_spd(state, idx);

    // Tick Roar Rumbles on Castorice turns
    let rem = state.stacks.get("castorice_roar_rem").copied().unwrap_or(0.0);
    if rem > 0.0 {
        let new = rem - 1.0;
        if new <= 0.0 {
            for i in 0..state.team.len() {
                if !state.team[i].is_downed { state.team[i].buffs.dmg_boost -= 10.0; }
            }
            state.stacks.remove("castorice_roar_rem");
        } else {
            state.stacks.insert("castorice_roar_rem".to_string(), new);
        }
    }
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    let max_hp = state.team[idx].max_hp;
    match action.action_type {
        ActionType::Basic => {
            action.scaling_stat_id  = ids::CHAR_HP_ID.to_string();
            action.multiplier       = 0.0;
            action.extra_dmg        = max_hp * 0.50;
            action.toughness_damage = 10.0;
            // E6: Quantum RES PEN
            if state.team[idx].eidolon >= 6 {
                state.team[idx].buffs.res_pen += 20.0;
            }
        }
        ActionType::Skill => {
            if is_nw_alive(state) {
                // Boneclaw: all damage handled in on_after_action
                action.multiplier       = 0.0;
                action.extra_dmg        = 0.0;
                action.toughness_damage = 0.0;
            } else {
                // Normal skill: main hit 50% HP
                action.scaling_stat_id  = ids::CHAR_HP_ID.to_string();
                action.multiplier       = 0.0;
                action.extra_dmg        = max_hp * 0.50;
                action.toughness_damage = 20.0;
            }
            if state.team[idx].eidolon >= 6 {
                state.team[idx].buffs.res_pen += 20.0;
            }
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
    state.team[idx].energy = 0.0;

    match action.action_type {
        ActionType::Basic => {
            // No HP consumption for basic; no Newbud from basic (only ally HP loss gives Newbud)
        }

        ActionType::Skill => {
            if is_nw_alive(state) {
                // Boneclaw: AoE joint attack
                let max_hp = state.team[idx].max_hp;
                let eidolon = state.team[idx].eidolon;

                // Castorice hits all enemies: 30% HP
                if eidolon >= 6 { state.team[idx].buffs.res_pen += 20.0; }
                let cast_dmg = aoe_hp_hit(state, idx, max_hp * 0.30, 20.0, ActionType::Skill);
                if eidolon >= 6 { state.team[idx].buffs.res_pen -= 20.0; }

                // Netherwing hits all enemies: 50% HP (uses Castorice stats)
                if eidolon >= 6 { state.team[idx].buffs.res_pen += 20.0; }
                let nw_dmg  = aoe_hp_hit(state, idx, max_hp * 0.50, 20.0, ActionType::TalentProc);
                if eidolon >= 6 { state.team[idx].buffs.res_pen -= 20.0; }

                // Consume 40% HP from all allies (except Netherwing — not in team)
                let consumed = consume_ally_hp(state, 0.40);
                // HP consumed → Netherwing HP recovery (not Newbud)
                if consumed > 0.0 {
                    add_nw_hp(state, consumed);
                    add_dmg_stack(state, idx);
                }

                let name = state.team[idx].name.clone();
                state.add_log(&name, format!(
                    "Boneclaw: {:.0} (Casto 30%) + {:.0} (NW 50%) AoE", cast_dmg, nw_dmg,
                ));
            } else {
                // Normal skill: adjacent hits (30% HP) + consume 30% HP
                let max_hp = state.team[idx].max_hp;
                if let Some(t) = target_idx {
                    let len  = state.enemies.len();
                    let eidolon = state.team[idx].eidolon;
                    let adj_action = ActionParams {
                        action_type:      ActionType::Skill,
                        scaling_stat_id:  ids::CHAR_HP_ID.to_string(),
                        multiplier:       0.0,
                        extra_multiplier: 0.0,
                        extra_dmg:        max_hp * 0.30,
                        toughness_damage: 10.0,
                        inflicts_debuff:  false,
                        is_ult_dmg:       false,
                    };
                    let member = state.team[idx].clone();
                    for &adj in &[if t > 0 { Some(t - 1) } else { None },
                                  if t + 1 < len { Some(t + 1) } else { None }]
                    {
                        let Some(a) = adj else { continue };
                        if state.enemies[a].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }
                        let e1_mult = if eidolon >= 1 {
                            let e_hp = state.enemies[a].as_ref().map(|e| e.hp).unwrap_or(0.0);
                            let cmax = member.max_hp;
                            if e_hp <= cmax * 0.50 { 1.40 }
                            else if e_hp <= cmax * 0.80 { 1.20 }
                            else { 1.0 }
                        } else { 1.0 };
                        let dmg = state.enemies[a].as_ref()
                            .map(|e| damage::calculate_damage(&member, e, &adj_action) * e1_mult)
                            .unwrap_or(0.0);
                        if dmg > 0.0 {
                            if let Some(e) = state.enemies[a].as_mut() { e.hp -= dmg; }
                            state.total_damage += dmg;
                        }
                        if state.enemies[a].as_ref().map_or(false, |e| e.hp <= 0.0) {
                            state.enemies[a] = None;
                        }
                    }
                }

                // Consume 30% HP from all allies
                let consumed = consume_ally_hp(state, 0.30);
                if consumed > 0.0 {
                    add_dmg_stack(state, idx);
                    add_newbud(state, idx, consumed);
                }
            }
        }
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].stacks.remove("_ult_ready");
    state.team[idx].energy = 0.0;

    // If Netherwing somehow still alive (rare edge case), dismiss it first
    if is_nw_alive(state) {
        let ws = nw_wings_sweep(state, idx);
        dismiss_netherwing(state, idx);
        let name = state.team[idx].name.clone();
        state.add_log(&name, format!("Pre-summon Wings Sweep: {:.0}", ws));
    }

    // Consume Newbud
    state.stacks.insert("castorice_newbud".to_string(), 0.0);

    let eidolon  = state.team[idx].eidolon;
    let nw_max   = state.stacks.get("castorice_newbud_max").copied().unwrap_or(9600.0);
    let gen      = state.stacks.get("netherwing_gen").copied().unwrap_or(0.0) + 1.0;

    state.stacks.insert("netherwing_alive".to_string(),       1.0);
    state.stacks.insert("netherwing_gen".to_string(),         gen);
    state.stacks.insert("netherwing_countdown".to_string(),   3.0);
    state.stacks.insert("netherwing_hp".to_string(),          nw_max);
    state.stacks.insert("netherwing_max_hp".to_string(),      nw_max);
    state.stacks.insert("netherwing_spd".to_string(),         165.0);
    state.stacks.insert("castorice_nw_claw_used".to_string(), 0.0);
    state.stacks.insert("netherwing_breath_seq".to_string(),  0.0);

    // E2: 2 Ardent Will stacks
    if eidolon >= 2 {
        state.stacks.insert("castorice_e2_ardent".to_string(), 2.0);
    }

    // Roar Rumbles the Realm: all allies +10% DMG for 3 turns
    for i in 0..state.team.len() {
        if !state.team[i].is_downed { state.team[i].buffs.dmg_boost += 10.0; }
    }
    state.stacks.insert("castorice_roar_rem".to_string(), 3.0);

    // Territory "Lost Netherland": all enemies -20% All-Type RES
    for slot in state.enemies.iter_mut() {
        if let Some(e) = slot.as_mut() {
            effects::apply_enemy_debuff(e, "castorice_territory", StatusEffect {
                duration: 999,
                value:    20.0,
                stat:     Some("All RES".to_string()),
                effects:  vec![],
            });
        }
    }

    // Talent DMG boost extends to Netherwing automatically (uses Castorice's buffs)

    // Netherwing acts immediately (100% action advance)
    state.av_queue.push(ActorEntry {
        next_av:     state.current_av,
        actor_id:    ids::NETHERWING_ID.to_string(),
        instance_id: gen.to_string(),
        is_enemy:    false,
    });

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Doomshriek: Netherwing summoned (HP:{:.0} = max Newbud, SPD:165)",
        nw_max,
    ));
}

#[allow(dead_code)]
pub fn on_break(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
