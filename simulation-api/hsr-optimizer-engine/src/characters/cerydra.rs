use crate::damage;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState};

// ── State key helpers (dynamic String keys → state.stacks) ───────────────────
fn spd_rem_key(i: usize) -> String { format!("cerydra_spd_rem_{i}") }

// ── ATK stat helper ──────────────────────────────────────────────────────────
fn effective_atk(member: &crate::models::TeamMember) -> f64 {
    let base = member.base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0)
             + member.base_stats.get(ids::LC_ATK_ID).copied().unwrap_or(0.0);
    base * (1.0 + member.buffs.atk_percent / 100.0) + member.buffs.atk_flat  // atk_flat added after scaling
}

// ── A2: CRIT DMG scaling from ATK above 2000 ─────────────────────────────────
fn a2_crit_dmg(atk: f64) -> f64 {
    if atk <= 2000.0 { return 0.0; }
    let excess_stacks = ((atk - 2000.0) / 100.0).floor();
    (excess_stacks * 18.0).min(360.0)
}

// ── Charge management ─────────────────────────────────────────────────────────
fn add_charge(state: &mut SimState, c_idx: usize, amount: f64) {
    let current = state.stacks.get("cerydra_charge").copied().unwrap_or(0.0);
    let peerage  = state.stacks.get("cerydra_peerage").copied().unwrap_or(0.0);
    let mm_val   = state.stacks.get("cerydra_mm_idx").copied().unwrap_or(-1.0);
    let new_ch   = (current + amount).min(8.0);
    state.stacks.insert("cerydra_charge".to_string(), new_ch);

    // Upgrade to Peerage when crossing threshold 6 while Peerage is not yet active
    if peerage < 1.0 && mm_val >= 0.0 && new_ch >= 6.0 {
        let mm = mm_val as usize;
        if mm < state.team.len() && !state.team[mm].is_downed {
            apply_peerage_buffs(state, c_idx, mm);
            state.stacks.insert("cerydra_peerage".to_string(), 1.0);
            let cname = state.team[c_idx].name.clone();
            let mname = state.team[mm].name.clone();
            state.add_log(&cname, format!("Military Merit → Peerage for {} (Charge {:.0})", mname, new_ch));
        }
    }
}

// ── MM buff helpers ───────────────────────────────────────────────────────────

fn apply_mm_buffs(state: &mut SimState, c_idx: usize, mm_idx: usize) {
    let eidolon = state.team[c_idx].eidolon;
    // Talent: MM target ATK += 24% of Cerydra's ATK (flat)
    let atk_buff = effective_atk(&state.team[c_idx]) * 0.24;
    state.team[mm_idx].buffs.atk_flat += atk_buff;
    state.stacks.insert("cerydra_mm_atk_applied".to_string(), atk_buff);
    // E1: MM target ignores 16% DEF
    if eidolon >= 1 { state.team[mm_idx].buffs.def_ignore += 16.0; }
    // E2: MM target +40% DMG
    if eidolon >= 2 { state.team[mm_idx].buffs.dmg_boost  += 40.0; }
    // E6: MM target +20% All-Type RES PEN
    if eidolon >= 6 { state.team[mm_idx].buffs.res_pen    += 20.0; }
    // Cerydra self-buffs while MM is active on a teammate
    if eidolon >= 2 { state.team[c_idx].buffs.dmg_boost += 160.0; }  // E2
    if eidolon >= 6 { state.team[c_idx].buffs.res_pen    += 20.0; }   // E6
}

fn remove_mm_buffs(state: &mut SimState, c_idx: usize, mm_idx: usize) {
    let eidolon  = state.team[c_idx].eidolon;
    let atk_buff = state.stacks.remove("cerydra_mm_atk_applied").unwrap_or(0.0);
    state.team[mm_idx].buffs.atk_flat -= atk_buff;
    if eidolon >= 1 { state.team[mm_idx].buffs.def_ignore -= 16.0; }
    if eidolon >= 2 { state.team[mm_idx].buffs.dmg_boost  -= 40.0; }
    if eidolon >= 6 { state.team[mm_idx].buffs.res_pen    -= 20.0; }
    if eidolon >= 2 { state.team[c_idx].buffs.dmg_boost -= 160.0; }
    if eidolon >= 6 { state.team[c_idx].buffs.res_pen    -= 20.0; }
}

fn apply_peerage_buffs(state: &mut SimState, c_idx: usize, mm_idx: usize) {
    let eidolon = state.team[c_idx].eidolon;
    // Peerage: +72% CRIT DMG (technically Skill-only but modelled globally), +10% RES PEN
    state.team[mm_idx].buffs.crit_dmg += 72.0;
    state.team[mm_idx].buffs.res_pen  += 10.0;
    // E1: Peerage Skill DMG additionally ignores 20% DEF
    if eidolon >= 1 { state.team[mm_idx].buffs.def_ignore += 20.0; }
}

fn remove_peerage_buffs(state: &mut SimState, c_idx: usize, mm_idx: usize) {
    let eidolon = state.team[c_idx].eidolon;
    state.team[mm_idx].buffs.crit_dmg -= 72.0;
    state.team[mm_idx].buffs.res_pen  -= 10.0;
    if eidolon >= 1 { state.team[mm_idx].buffs.def_ignore -= 20.0; }
}

// ── Grant Military Merit to a target ─────────────────────────────────────────
fn grant_mm(state: &mut SimState, c_idx: usize, new_mm: usize) {
    let old_mm_val = state.stacks.get("cerydra_mm_idx").copied().unwrap_or(-1.0);
    let old_mm     = old_mm_val as i64;
    let new_mm_i   = new_mm as i64;

    if old_mm == new_mm_i {
        // Same target: refresh without re-applying buffs
        let cname = state.team[c_idx].name.clone();
        let mname = state.team[new_mm].name.clone();
        state.add_log(&cname, format!("Military Merit refresh → {}", mname));
        return;
    }

    // Target is changing: revoke old MM/Peerage and A6 SPD
    if old_mm >= 0 {
        let old = old_mm as usize;
        if old < state.team.len() {
            let had_peerage = state.stacks.get("cerydra_peerage").copied().unwrap_or(0.0) >= 1.0;
            if had_peerage {
                remove_peerage_buffs(state, c_idx, old);
                state.stacks.insert("cerydra_peerage".to_string(), 0.0);
            }
            remove_mm_buffs(state, c_idx, old);
            // Revoke A6 SPD from old MM target
            let old_rem = state.stacks.remove(&spd_rem_key(old)).unwrap_or(0.0);
            if old_rem > 0.0 {
                let cur = state.team[old].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
                state.team[old].base_stats.insert(ids::CHAR_SPD_ID.to_string(), (cur - 20.0).max(0.0));
            }
        }
    }

    // Reset Charge when target changes
    state.stacks.insert("cerydra_charge".to_string(), 0.0);
    state.stacks.insert("cerydra_mm_idx".to_string(), new_mm as f64);
    apply_mm_buffs(state, c_idx, new_mm);

    let cname = state.team[c_idx].name.clone();
    let mname = state.team[new_mm].name.clone();
    state.add_log(&cname, format!("Military Merit → {}", mname));
}

// ── Pick best MM target (highest ATK non-Cerydra alive ally) ─────────────────
fn best_mm_target(state: &SimState, c_idx: usize) -> Option<usize> {
    (0..state.team.len())
        .filter(|&i| i != c_idx && !state.team[i].is_downed)
        .max_by(|&a, &b| {
            let a_atk = state.team[a].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
            let b_atk = state.team[b].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
            a_atk.partial_cmp(&b_atk).unwrap_or(std::cmp::Ordering::Equal)
        })
}

// ── SPD buff tick (A6) ────────────────────────────────────────────────────────
fn tick_spd_buff(state: &mut SimState, i: usize, self_buff: bool) {
    let key = if self_buff { "cerydra_spd_self_rem".to_string() } else { spd_rem_key(i) };
    let rem = state.stacks.get(&key).copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }
    if rem <= 1.0 {
        state.stacks.remove(&key);
        let cur = state.team[i].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        state.team[i].base_stats.insert(ids::CHAR_SPD_ID.to_string(), (cur - 20.0).max(0.0));
    } else {
        state.stacks.insert(key, rem - 1.0);
    }
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 130.0;

    // Minor traces
    state.team[idx].buffs.atk_percent += 18.0;
    state.team[idx].buffs.dmg_boost   += 22.4; // Wind DMG +22.4%
    state.team[idx].buffs.hp_percent  += 10.0;

    // A4: CRIT Rate +100%
    state.team[idx].buffs.crit_rate += 100.0;

    // A2: CRIT DMG from ATK (computed once at battle start; ATK traces applied above)
    let atk  = effective_atk(&state.team[idx]);
    let a2cd = a2_crit_dmg(atk);
    state.team[idx].buffs.crit_dmg += a2cd;
    state.stacks.insert("cerydra_a2_cd_applied".to_string(), a2cd);

    // Init state
    state.stacks.insert("cerydra_charge".to_string(),     0.0);
    state.stacks.insert("cerydra_mm_idx".to_string(),    -1.0);
    state.stacks.insert("cerydra_peerage".to_string(),    0.0);
    state.stacks.insert("cerydra_addl_count".to_string(), 0.0);
    state.stacks.insert("cerydra_a4_used".to_string(),    0.0);

    // Technique: auto-Skill at battle start on best ally (+1 Charge, no SP cost)
    if let Some(t) = best_mm_target(state, idx) {
        grant_mm(state, idx, t);
        add_charge(state, idx, 1.0);
        let cname = state.team[idx].name.clone();
        state.add_log(&cname, "Technique: auto-Skill at battle start (Military Merit granted)".to_string());
    }
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // Tick Cerydra's own A6 SPD buff
    tick_spd_buff(state, idx, true);
}

pub fn on_before_action(
    _state: &mut SimState,
    _idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Skill => {
            // Support skill: no damage
            action.multiplier       = 0.0;
            action.toughness_damage = 0.0;
        }
        ActionType::Ultimate => {
            // AoE handled entirely in on_ult via _ult_handled; suppress default damage
            action.inflicts_debuff = false;
        }
        _ => {}
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    _target_idx: Option<usize>,
) {
    if action.action_type != ActionType::Skill { return; }

    let e1 = state.team[idx].eidolon >= 1;

    if let Some(t) = best_mm_target(state, idx) {
        grant_mm(state, idx, t);

        // E1: +2 Energy for MM target on Skill use
        if e1 {
            let err  = 1.0 + state.team[t].buffs.energy_regen_rate / 100.0;
            let maxe = state.team[t].max_energy;
            state.team[t].energy = (state.team[t].energy + 2.0 * err).min(maxe);
        }

        // A6: +20 SPD to Cerydra for 3 turns (no re-stack if already active)
        let self_rem = state.stacks.get("cerydra_spd_self_rem").copied().unwrap_or(0.0);
        if self_rem <= 0.0 {
            let spd = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
            state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), spd + 20.0);
        }
        state.stacks.insert("cerydra_spd_self_rem".to_string(), 3.0);

        // A6: +20 SPD to MM ally for 3 turns
        let ally_key = spd_rem_key(t);
        let ally_rem = state.stacks.get(&ally_key).copied().unwrap_or(0.0);
        if ally_rem <= 0.0 {
            let spd = state.team[t].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
            state.team[t].base_stats.insert(ids::CHAR_SPD_ID.to_string(), spd + 20.0);
        }
        state.stacks.insert(ally_key, 3.0);

        // Gain 1 Charge from Skill use
        add_charge(state, idx, 1.0);

        let charge = state.stacks.get("cerydra_charge").copied().unwrap_or(0.0);
        let cname  = state.team[idx].name.clone();
        let mname  = state.team[t].name.clone();
        state.add_log(&cname, format!(
            "Pawn's Promotion: MM → {}, Charge → {:.0}, A6 SPD+20 (3t){}",
            mname, charge,
            if e1 { ", E1 +2 Energy" } else { "" }
        ));
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0; // Ult energy gain

    let e4 = state.team[idx].eidolon >= 4;
    let multiplier = if e4 { 4.80 } else { 2.40 };

    // Gain 2 Charge (may upgrade to Peerage)
    add_charge(state, idx, 2.0);

    // Reset Additional DMG count each ult
    state.stacks.insert("cerydra_addl_count".to_string(), 0.0);

    // If no MM active, grant to first alive non-self ally
    if state.stacks.get("cerydra_mm_idx").copied().unwrap_or(-1.0) < 0.0 {
        let first = (0..state.team.len()).find(|&i| i != idx && !state.team[i].is_downed);
        if let Some(t) = first {
            grant_mm(state, idx, t);
        }
    }

    // AoE Wind DMG to all enemies
    let member = state.team[idx].clone();
    let ult_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();

    let mut total_dmg = 0.0f64;
    for &i in &alive {
        let dmg = state.enemies[i].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &ult_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[i].as_mut() { e.hp -= dmg; }
            total_dmg += dmg;
        }
        if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[i] = None;
        }
    }
    state.total_damage += total_dmg;

    let charge = state.stacks.get("cerydra_charge").copied().unwrap_or(0.0);
    let name   = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Scholar's Mate: AoE {:.0}% ATK Wind, {:.0} DMG (Charge → {:.0}){}",
        multiplier * 100.0, total_dmg, charge,
        if e4 { " [E4 +240%]" } else { "" }
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
    state: &mut SimState,
    idx: usize,        // Cerydra's team index
    source_idx: usize, // ally who just acted
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // Only react to the ally who holds Military Merit
    let mm_val = state.stacks.get("cerydra_mm_idx").copied().unwrap_or(-1.0);
    if mm_val < 0.0 || (mm_val as usize) != source_idx {
        return;
    }

    let mm      = source_idx;
    let peerage = state.stacks.get("cerydra_peerage").copied().unwrap_or(0.0) >= 1.0;
    let is_attack = matches!(action.action_type,
        ActionType::Basic | ActionType::Skill | ActionType::FollowUp | ActionType::TalentProc);
    let is_basic_or_skill = matches!(action.action_type, ActionType::Basic | ActionType::Skill);
    let is_skill  = action.action_type == ActionType::Skill;
    let is_ult    = action.action_type == ActionType::Ultimate;
    // Coup de Main = Peerage ally uses Skill
    let is_coup   = is_skill && peerage;

    // A6: tick MM ally's SPD buff on their action
    tick_spd_buff(state, mm, false);

    if is_attack {
        // Talent: Additional Wind DMG (up to 20 per ult; E6: 360% ATK, else 60% ATK)
        let addl_count = state.stacks.get("cerydra_addl_count").copied().unwrap_or(0.0);
        if addl_count < 20.0 {
            let t_slot = target_idx.unwrap_or_else(|| {
                state.enemies.iter()
                    .position(|s| s.as_ref().map_or(false, |e| e.hp > 0.0))
                    .unwrap_or(0)
            });
            if state.enemies.get(t_slot).and_then(|s| s.as_ref()).map_or(false, |e| e.hp > 0.0) {
                let e6        = state.team[idx].eidolon >= 6;
                let addl_mult = if e6 { 3.60 } else { 0.60 };
                let member    = state.team[idx].clone();
                let addl_action = ActionParams {
                    action_type:      ActionType::TalentProc,
                    scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                    multiplier:       addl_mult,
                    extra_multiplier: 0.0,
                    extra_dmg:        0.0,
                    toughness_damage: 0.0,
                    inflicts_debuff:  false,
                    is_ult_dmg:       false,
                };
                let dmg = state.enemies[t_slot].as_ref()
                    .map(|e| damage::calculate_damage(&member, e, &addl_action))
                    .unwrap_or(0.0);
                if dmg > 0.0 {
                    if let Some(e) = state.enemies[t_slot].as_mut() { e.hp -= dmg; }
                    state.total_damage += dmg;
                    if state.enemies[t_slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
                        state.enemies[t_slot] = None;
                    }
                }
                state.stacks.insert("cerydra_addl_count".to_string(), addl_count + 1.0);
                let cname = state.team[idx].name.clone();
                state.add_log(&cname, format!(
                    "Ave Imperator: {:.0}% ATK Wind Additional, {:.0} DMG [{:.0}/20]",
                    addl_mult * 100.0, dmg, addl_count + 1.0
                ));
            }
        }

        // Talent: Cerydra gains 1 Charge (blocked during Coup de Main)
        if !is_coup {
            add_charge(state, idx, 1.0);
        }

        // A6: +5 Energy for Cerydra when MM ally uses Basic or Skill
        if is_basic_or_skill {
            let err  = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
            let maxe = state.team[idx].max_energy;
            state.team[idx].energy = (state.team[idx].energy + 5.0 * err).min(maxe);
        }
    }

    // A4: MM ally's Ultimate grants Cerydra 1 Charge (once per battle, while Charge < 8)
    if is_ult {
        let a4_used = state.stacks.get("cerydra_a4_used").copied().unwrap_or(0.0);
        let charge  = state.stacks.get("cerydra_charge").copied().unwrap_or(0.0);
        if a4_used < 1.0 && charge < 8.0 {
            add_charge(state, idx, 1.0);
            state.stacks.insert("cerydra_a4_used".to_string(), 1.0);
            let cname = state.team[idx].name.clone();
            state.add_log(&cname, "A4: MM ally Ult → +1 Charge (once per battle)".to_string());
        }
    }

    // Coup de Main end: consume 6 Charge, revert Peerage → Military Merit
    if is_coup {
        let charge = state.stacks.get("cerydra_charge").copied().unwrap_or(0.0);
        let new_ch = (charge - 6.0).max(0.0);
        state.stacks.insert("cerydra_charge".to_string(), new_ch);
        remove_peerage_buffs(state, idx, mm);
        state.stacks.insert("cerydra_peerage".to_string(), 0.0);
        let cname = state.team[idx].name.clone();
        let mname = state.team[mm].name.clone();
        state.add_log(&cname, format!(
            "Coup de Main end: {}'s Peerage → MM (Charge {:.0} → {:.0})",
            mname, charge, new_ch
        ));
    }
}
