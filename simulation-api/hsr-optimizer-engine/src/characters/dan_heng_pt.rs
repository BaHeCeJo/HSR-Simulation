use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, ActorEntry, SimState, StatusEffect};

// ─── State keys ───────────────────────────────────────────────────────────────

// Global (state.stacks, String keys — cross-character / summon state)
const SD_ALIVE:     &str = "dhpt_sd_alive";     // 1.0 if Souldragon is alive
const SD_GEN:       &str = "dhpt_sd_gen";        // generation counter (stale AV-entry guard)
const SD_ENHANCED:  &str = "dhpt_sd_enhanced";   // remaining enhanced Souldragon actions
const BONDMATE_IDX: &str = "dhpt_bondmate_idx";  // team index of current Bondmate
const SD_AV_REDUCE: &str = "dhpt_sd_av_reduce";  // pending AV advance for Souldragon (A4)

// Per-character (state.team[idx].stacks, &'static str keys)
const E1_RES_REM: &str = "dhpt_e1_rem"; // turns of E1 +18% RES PEN remaining on Bondmate
const E1_BM_IDX:  &str = "dhpt_e1_bm";  // Bondmate index when E1 was applied (for revert)

// ─── Helpers ──────────────────────────────────────────────────────────────────

pub fn is_sd_alive(state: &SimState) -> bool {
    state.stacks.get(SD_ALIVE).copied().unwrap_or(0.0) >= 1.0
}

fn get_bondmate(state: &SimState) -> Option<usize> {
    let v = state.stacks.get(BONDMATE_IDX).copied().unwrap_or(-1.0);
    if v < 0.0 { return None; }
    let i = v as usize;
    if i < state.team.len() && !state.team[i].is_downed { Some(i) } else { None }
}

fn pick_dps_ally(state: &SimState, dhpt_idx: usize) -> Option<usize> {
    // Pick the lowest-aggro-path ally (the DPS) to designate as Bondmate
    fn weight(path: &str) -> i32 {
        match path {
            "Preservation" => 6,
            "Destruction"  => 5,
            "Harmony" | "Nihility" | "Abundance" | "Remembrance" | "Elation" => 4,
            _              => 3, // The Hunt, Erudition
        }
    }
    state.team.iter().enumerate()
        .filter(|(i, m)| *i != dhpt_idx && !m.is_downed)
        .min_by_key(|(_, m)| weight(&m.path))
        .map(|(i, _)| i)
}

fn apply_e6_vuln(state: &mut SimState) {
    for slot in state.enemies.iter_mut() {
        if let Some(e) = slot.as_mut() {
            effects::apply_enemy_buff(e, "dhpt_e6_vuln", StatusEffect {
                duration: 999,
                value:    20.0,
                stat:     Some("Vulnerability".to_string()),
                effects:  vec![],
            });
        }
    }
}

fn summon_souldragon(state: &mut SimState, dhpt_idx: usize) {
    let gen = state.stacks.get(SD_GEN).copied().unwrap_or(0.0) + 1.0;
    state.stacks.insert(SD_ALIVE.to_string(),    1.0);
    state.stacks.insert(SD_GEN.to_string(),      gen);
    state.stacks.insert(SD_ENHANCED.to_string(), 0.0);

    state.av_queue.push(ActorEntry {
        next_av:     state.current_av + 10000.0 / 165.0,
        actor_id:    ids::SOULDRAGON_ID.to_string(),
        instance_id: gen.to_string(),
        is_enemy:    false,
    });

    let name = state.team[dhpt_idx].name.clone();
    state.add_log(&name, "Souldragon summoned (SPD: 165)".to_string());
}

/// Designate the Bondmate (once at battle start / Technique pre-battle).
/// Applies A2 flat ATK bonus, E4/E6 permanent buffs, and summons Souldragon.
fn designate_bondmate(state: &mut SimState, dhpt_idx: usize) {
    let bm = match pick_dps_ally(state, dhpt_idx) {
        Some(i) => i,
        None    => return,
    };
    state.stacks.insert(BONDMATE_IDX.to_string(), bm as f64);

    // A2: Bondmate gains flat ATK = 15% of DHPT's total ATK
    let base = state.team[dhpt_idx].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
    let lc   = state.team[dhpt_idx].lightcone.base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
    let pct  = state.team[dhpt_idx].buffs.atk_percent;
    let flat = state.team[dhpt_idx].buffs.atk_flat;
    let a2   = ((base + lc) * (1.0 + pct / 100.0) + flat) * 0.15;
    state.team[bm].buffs.atk_flat += a2;

    // E4: Bondmate takes 20% less incoming DMG (defensive, doesn't affect DPS sim)
    if state.team[dhpt_idx].eidolon >= 4 {
        state.team[bm].buffs.incoming_dmg_reduction += 20.0;
    }

    // E6: all enemies take +20% vulnerability; Bondmate ignores 12% DEF
    if state.team[dhpt_idx].eidolon >= 6 {
        apply_e6_vuln(state);
        state.team[bm].buffs.def_ignore += 12.0;
    }

    summon_souldragon(state, dhpt_idx);

    let bm_name = state.team[bm].name.clone();
    let name    = state.team[dhpt_idx].name.clone();
    state.add_log(&name, format!("Bondmate → {} (A2: +{:.0} flat ATK)", bm_name, a2));
}

// ─── Souldragon turn (called by simulator.rs) ─────────────────────────────────

pub fn souldragon_turn(state: &mut SimState, dhpt_idx: usize) {
    if !is_sd_alive(state) { return; }

    let eidolon      = state.team[dhpt_idx].eidolon;
    let enhanced_rem = state.stacks.get(SD_ENHANCED).copied().unwrap_or(0.0);
    let is_enhanced  = enhanced_rem > 0.0;

    if is_enhanced {
        state.stacks.insert(SD_ENHANCED.to_string(), (enhanced_rem - 1.0).max(0.0));
    } else {
        // Non-enhanced: shield + debuff cleanse — no damage relevant to the sim
        let name = state.team[dhpt_idx].name.clone();
        state.add_log(&name, "Souldragon: Shield + cleanse (no DMG)".to_string());
        return;
    }

    // ── Enhanced FUA: Physical AoE (DHPT ATK) + Bondmate-element AoE ─────────

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if alive.is_empty() { return; }

    // Physical: 80% DHPT ATK to all enemies
    let dhpt_member = state.team[dhpt_idx].clone();
    let phys_action = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       0.80,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };
    let mut phys_total = 0.0;
    for &s in &alive {
        let dmg = state.enemies[s].as_ref()
            .map(|e| damage::calculate_damage(&dhpt_member, e, &phys_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[s].as_mut() { e.hp -= dmg; }
            phys_total += dmg;
        }
        if state.enemies[s].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[s] = None;
        }
    }
    state.total_damage += phys_total;

    // Bondmate-element: 80% Bondmate ATK AoE (E2: 160%)
    let bm_mult = if eidolon >= 2 { 1.60 } else { 0.80 };
    let mut bm_total = 0.0;
    let mut a6_total = 0.0;

    if let Some(bi) = get_bondmate(state) {
        let mut bm_member = state.team[bi].clone();
        // E6: Bondmate ignores 12% DEF (already baked into buffs but re-assert for clone)
        if eidolon >= 6 { bm_member.buffs.def_ignore += 12.0; }

        let bm_action = ActionParams {
            action_type:      ActionType::FollowUp,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       bm_mult,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 20.0,
            inflicts_debuff:  false,
            is_ult_dmg:       false,
        };
        let alive2: Vec<usize> = state.enemies.iter().enumerate()
            .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
            .collect();
        for &s in &alive2 {
            let dmg = state.enemies[s].as_ref()
                .map(|e| damage::calculate_damage(&bm_member, e, &bm_action))
                .unwrap_or(0.0);
            if dmg > 0.0 {
                if let Some(e) = state.enemies[s].as_mut() { e.hp -= dmg; }
                bm_total += dmg;
            }
            if state.enemies[s].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[s] = None;
            }
        }
        state.total_damage += bm_total;

        // A6: extra 40% Bondmate ATK to highest-HP enemy when enhanced
        let a6_slot = state.enemies.iter().enumerate()
            .filter_map(|(i, s)| s.as_ref().filter(|e| e.hp > 0.0).map(|e| (i, e.hp)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(i, _)| i);
        if let Some(hs) = a6_slot {
            let a6_action = ActionParams {
                action_type:      ActionType::FollowUp,
                scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                multiplier:       0.40,
                extra_multiplier: 0.0,
                extra_dmg:        0.0,
                toughness_damage: 0.0,
                inflicts_debuff:  false,
                is_ult_dmg:       false,
            };
            let a6_dmg = state.enemies[hs].as_ref()
                .map(|e| damage::calculate_damage(&bm_member, e, &a6_action))
                .unwrap_or(0.0);
            if a6_dmg > 0.0 {
                if let Some(e) = state.enemies[hs].as_mut() { e.hp -= a6_dmg; }
                if state.enemies[hs].as_ref().map_or(false, |e| e.hp <= 0.0) {
                    state.enemies[hs] = None;
                }
                a6_total += a6_dmg;
                state.total_damage += a6_total;
            }
        }
    }

    let name = state.team[dhpt_idx].name.clone();
    state.add_log(&name, format!(
        "Souldragon Enhanced: Phys {:.0} + BM {:.0} + A6 {:.0} DMG",
        phys_total, bm_total, a6_total
    ));
}

// ─── Character hooks ──────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 135.0;
    state.team[idx].buffs.atk_percent += 28.0;   // minor trace ATK%
    state.team[idx].buffs.def_percent += 22.5;   // minor trace DEF%
    // +5 flat SPD minor trace
    let cur_spd = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), cur_spd + 5.0);

    state.stacks.insert(SD_ALIVE.to_string(),     0.0);
    state.stacks.insert(SD_GEN.to_string(),       0.0);
    state.stacks.insert(SD_ENHANCED.to_string(),  0.0);
    state.stacks.insert(SD_AV_REDUCE.to_string(), 0.0);

    // Technique: free Skill at battle start → pick Bondmate and summon Souldragon
    designate_bondmate(state, idx);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // E1: tick down RES PEN buff on Bondmate
    if state.team[idx].eidolon < 1 { return; }

    let rem = state.team[idx].stacks.get(E1_RES_REM).copied().unwrap_or(0.0);
    if rem <= 0.0 { return; }

    if rem <= 1.0 {
        state.team[idx].stacks.remove(E1_RES_REM);
        let bm_v = state.team[idx].stacks.get(E1_BM_IDX).copied().unwrap_or(-1.0);
        state.team[idx].stacks.remove(E1_BM_IDX);
        if bm_v >= 0.0 {
            let bi = bm_v as usize;
            if bi < state.team.len() {
                state.team[bi].buffs.res_pen -= 18.0;
            }
        }
    } else {
        state.team[idx].stacks.insert(E1_RES_REM, rem - 1.0);
    }
}

pub fn on_before_action(
    _state:  &mut SimState,
    _idx:    usize,
    action:  &mut ActionParams,
    _target: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic    => { action.multiplier = 1.00; action.toughness_damage = 10.0; }
        ActionType::Skill    => { action.multiplier = 0.0;  action.toughness_damage = 0.0; }
        ActionType::Ultimate => { action.multiplier = 0.0;  action.toughness_damage = 0.0; }
        _ => {}
    }
}

pub fn on_after_action(
    state:   &mut SimState,
    idx:     usize,
    action:  &ActionParams,
    _target: Option<usize>,
) {
    // Skill: re-summon Souldragon if it somehow disappeared
    if action.action_type == ActionType::Skill && !is_sd_alive(state) {
        summon_souldragon(state, idx);
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    let eidolon = state.team[idx].eidolon;

    // E1: recover 1 SP
    if eidolon >= 1 {
        state.skill_points = (state.skill_points + 1).min(5);
    }

    // AoE 300% ATK Physical to all enemies
    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();

    let member     = state.team[idx].clone();
    let ult_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       3.00,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };
    let mut ult_total = 0.0;
    for &s in &alive {
        let dmg = state.enemies[s].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &ult_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[s].as_mut() { e.hp -= dmg; }
            ult_total += dmg;
        }
        if state.enemies[s].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[s] = None;
        }
    }
    state.total_damage += ult_total;

    // Enhance Souldragon (2 base, E2: +2 = 4)
    let enhanced_count = if eidolon >= 2 { 4.0 } else { 2.0 };
    state.stacks.insert(SD_ENHANCED.to_string(), enhanced_count);

    // E1: +18% All-Type RES PEN on Bondmate for 3 turns
    if eidolon >= 1 {
        if let Some(bi) = get_bondmate(state) {
            // Remove old E1 buff if still active
            let old_rem = state.team[idx].stacks.get(E1_RES_REM).copied().unwrap_or(0.0);
            if old_rem > 0.0 {
                let old_bm = state.team[idx].stacks.get(E1_BM_IDX).copied().unwrap_or(-1.0);
                if old_bm >= 0.0 {
                    let obi = old_bm as usize;
                    if obi < state.team.len() {
                        state.team[obi].buffs.res_pen -= 18.0;
                    }
                }
            }
            state.team[bi].buffs.res_pen += 18.0;
            state.team[idx].stacks.insert(E1_RES_REM, 3.0);
            state.team[idx].stacks.insert(E1_BM_IDX,  bi as f64);
        }
    }

    // E2: Souldragon immediately advances action (push a current-AV entry)
    if eidolon >= 2 && is_sd_alive(state) {
        let gen = state.stacks.get(SD_GEN).copied().unwrap_or(0.0);
        state.av_queue.push(ActorEntry {
            next_av:     state.current_av,
            actor_id:    ids::SOULDRAGON_ID.to_string(),
            instance_id: gen.to_string(),
            is_enemy:    false,
        });
    }

    // E6: Bondmate deals 330% Bondmate ATK AoE immediately
    if eidolon >= 6 {
        if let Some(bi) = get_bondmate(state) {
            let alive2: Vec<usize> = state.enemies.iter().enumerate()
                .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
                .collect();
            if !alive2.is_empty() {
                // E6 also re-applies vulnerability in case new enemies appeared
                apply_e6_vuln(state);
                let mut bm_member = state.team[bi].clone();
                bm_member.buffs.def_ignore += 12.0; // E6 DEF ignore (already in buffs, extra for clone)
                let e6_action = ActionParams {
                    action_type:      ActionType::Ultimate,
                    scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                    multiplier:       3.30,
                    extra_multiplier: 0.0,
                    extra_dmg:        0.0,
                    toughness_damage: 0.0,
                    inflicts_debuff:  false,
                    is_ult_dmg:       true,
                };
                let mut e6_total = 0.0;
                for &s in &alive2 {
                    let dmg = state.enemies[s].as_ref()
                        .map(|e| damage::calculate_damage(&bm_member, e, &e6_action))
                        .unwrap_or(0.0);
                    if dmg > 0.0 {
                        if let Some(e) = state.enemies[s].as_mut() { e.hp -= dmg; }
                        e6_total += dmg;
                    }
                    if state.enemies[s].as_ref().map_or(false, |e| e.hp <= 0.0) {
                        state.enemies[s] = None;
                    }
                }
                state.total_damage += e6_total;
                let name = state.team[idx].name.clone();
                state.add_log(&name, format!("E6 Bondmate AoE (330%): {:.0} DMG", e6_total));
            }
        }
    }

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Ult AoE (300%): {:.0} DMG | Souldragon enhanced ×{:.0}{}",
        ult_total, enhanced_count,
        if eidolon >= 1 { " | +1 SP" } else { "" }
    ));
}

pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state:      &mut SimState,
    idx:        usize,
    source_idx: usize,
    action:     &ActionParams,
    _target:    Option<usize>,
) {
    // A4: when Bondmate uses an attack, DHPT gains 6 Energy and Souldragon advances 15%
    if let Some(bi) = get_bondmate(state) {
        if source_idx == bi && matches!(action.action_type,
            ActionType::Basic | ActionType::Skill | ActionType::Ultimate)
        {
            let err   = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
            let max_e = state.team[idx].max_energy;
            state.team[idx].energy = (state.team[idx].energy + 6.0 * err).min(max_e);

            if is_sd_alive(state) {
                let cur    = state.stacks.get(SD_AV_REDUCE).copied().unwrap_or(0.0);
                let reduce = 10000.0 / 165.0 * 0.15;
                state.stacks.insert(SD_AV_REDUCE.to_string(), cur + reduce);
            }
        }
    }
}
