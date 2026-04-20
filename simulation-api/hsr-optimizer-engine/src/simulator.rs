use std::collections::{BinaryHeap, HashMap};

use crate::characters;
use crate::lightcones;
use crate::damage;
use crate::effects;
use crate::enemies;
use crate::ids;
use crate::models::{
    AbilityLevels, ActionParams, ActionType, ActorEntry, Buffs, IncomingCharacter,
    IncomingLightcone, IncomingRelic, LightconeStats, SimEnemy, SimReport, SimState,
    TeamMember, Wave,
};
use crate::planars;
use crate::relics;

// ─── Rich debug logging helpers ──────────────────────────────────────────────

/// Emit a turn-start log entry for ally `idx` with stats, SP, energy, buffs/debuffs.
fn log_turn_start(state: &mut SimState, idx: usize) {
    if !state.with_logs { return; }

    // Pre-collect all values from the immutable borrow of state.team[idx]
    let (name, stats_line, buffs_line, debuffs_line, header_line) = {
        let m     = &state.team[idx];
        let atk_b = m.base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0)
                  + m.lightcone.base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
        let atk   = atk_b * (1.0 + m.buffs.atk_percent / 100.0);
        let def_b = m.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(0.0);
        let def   = def_b * (1.0 + m.buffs.def_percent / 100.0);
        let spd   = m.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0)
                  * (1.0 + m.buffs.speed_percent / 100.0);
        let energy = if m.max_energy > 1e12 { "custom".to_string() }
                     else { format!("{:.0}/{:.0}", m.energy, m.max_energy) };
        let shield_str = if m.shield > 0.0 { format!(" | Shield:{:.0}", m.shield) } else { String::new() };
        let header = format!("Turn start | SP:{} | Energy:{}", state.skill_points, energy);
        let stats  = format!(
            "ATK:{:.0} DEF:{:.0} | CR:{:.1}% | CD:{:.1}% | SPD:{:.0} | HP:{:.0}/{:.0}{}",
            atk, def, m.buffs.crit_rate, m.buffs.crit_dmg, spd, m.hp, m.max_hp, shield_str
        );
        let buffs = if m.active_buffs.is_empty() { None } else {
            Some(m.active_buffs.iter()
                .map(|(k, v)| format!("{}({}t)", k, v.duration))
                .collect::<Vec<_>>().join(", "))
        };
        let debuffs = if m.active_debuffs.is_empty() { None } else {
            Some(m.active_debuffs.iter()
                .map(|(k, v)| format!("{}({}t)", k, v.duration))
                .collect::<Vec<_>>().join(", "))
        };
        (m.name.clone(), stats, buffs, debuffs, header)
    };

    state.add_log(&name, header_line);
    state.add_log_sub(stats_line);

    // Warn when base stats appear missing (TypeScript server didn't send them)
    {
        let m = &state.team[idx];
        let has_atk = m.base_stats.contains_key(ids::CHAR_ATK_ID);
        let has_hp  = m.base_stats.contains_key(ids::CHAR_HP_ID);
        if !has_atk || !has_hp {
            let keys: Vec<&str> = m.base_stats.keys()
                .take(4)
                .map(|s| s.as_str())
                .collect();
            state.add_log_sub(format!(
                "WARN: stat payload incomplete ({} keys loaded, sample: {:?}). Check TypeScript server sends basic_stats for this character.",
                m.base_stats.len(), keys
            ));
        }
    }

    if let Some(b) = buffs_line   { state.add_log_sub(format!("Buffs: {}", b)); }
    if let Some(d) = debuffs_line { state.add_log_sub(format!("Debuffs: {}", d)); }
}

/// Emit a snapshot of all living enemies (HP, toughness, debuffs) as sub-entries on the last log.
fn log_enemy_snapshot(state: &mut SimState) {
    if !state.with_logs { return; }
    // Pre-collect all strings (immutable pass) then log (mutable pass)
    let lines: Vec<String> = state.enemies.iter()
        .filter_map(|slot| slot.as_ref())
        .filter(|e| e.hp > 0.0)
        .map(|e| {
            let debuffs = if e.active_debuffs.is_empty() {
                "no debuffs".to_string()
            } else {
                e.active_debuffs.iter()
                    .map(|(k, v)| format!("{}({:.1},{}t)", k, v.value, v.duration))
                    .collect::<Vec<_>>().join(", ")
            };
            format!(
                "{}: HP:{:.0}/{:.0} TGH:{:.0}/{:.0} | {}",
                e.name, e.hp, e.max_hp, e.toughness, e.max_toughness, debuffs
            )
        })
        .collect();
    for line in lines {
        state.add_log_sub(line);
    }
}

// ─── Aggro weights by path ────────────────────────────────────────────────────

fn base_aggro(path: &str) -> f64 {
    match path {
        "The Hunt" | "Erudition"                                => 3.0,
        "Harmony" | "Nihility" | "Abundance" | "Remembrance"
        | "Elation"                                             => 4.0,
        "Destruction"                                           => 5.0,
        "Preservation"                                          => 6.0,
        _                                                       => 4.0,
    }
}

// ─── Character → TeamMember mapping ──────────────────────────────────────────

fn map_to_team_member(
    char: &IncomingCharacter,
    lc:   Option<&IncomingLightcone>,
) -> TeamMember {
    // ── Load character stats ──────────────────────────────────────────────────
    let mut base_stats: HashMap<String, f64> = HashMap::new();
    if let Some(bs) = &char.basic_stats {
        for (id, sv) in bs { base_stats.insert(id.clone(), sv.value); }
    }
    if let Some(adv) = &char.advanced_stats {
        for (id, sv) in adv { base_stats.insert(id.clone(), sv.value); }
    }

    // ── Load lightcone stats ──────────────────────────────────────────────────
    let mut lc_stats: HashMap<String, f64> = HashMap::new();
    if let Some(lc) = lc {
        if let Some(bs) = &lc.basic_stats {
            for (id, sv) in bs { lc_stats.insert(id.clone(), sv.value); }
        }
        if let Some(adv) = &lc.advanced_stats {
            for (id, sv) in adv { lc_stats.insert(id.clone(), sv.value); }
        }
    }

    // ── Remap LC-specific stat UUIDs → character stat UUIDs ──────────────────
    // The TypeScript server stores LC ATK/DEF/HP under different UUIDs than
    // the character stat UUIDs used by the damage formula.
    // We merge them into the canonical CHAR_*_ID keys so calculate_damage
    // picks them up via lc_base = lightcone.base_stats[scaling_stat_id].
    const LC_DEF_UUID: &str = "52566b38-915c-4220-ab0e-61438225704b"; // aventurine.rs
    for (lc_uuid, char_uuid) in [
        (ids::LC_ATK_ID,  ids::CHAR_ATK_ID),
        (LC_DEF_UUID,     ids::CHAR_DEF_ID),
    ] {
        if let Some(v) = lc_stats.remove(lc_uuid) {
            *lc_stats.entry(char_uuid.to_string()).or_insert(0.0) += v;
        }
    }

    let lc_id = lc.and_then(|l| l.lightcone_id.clone()).unwrap_or_default();
    let lc_si = lc.and_then(|l| l.superimposition).unwrap_or(1);

    let element        = char.attribute.clone().unwrap_or_else(|| "Physical".to_string());
    let path           = char.path.clone().unwrap_or_else(|| "Hunt".to_string());
    let abilities      = char.abilities.clone().unwrap_or_default();
    let relic_list: Vec<IncomingRelic> = char.relics.clone().unwrap_or_default();

    // ── Character capability flags ────────────────────────────────────────────
    // has_memo: Remembrance-path characters that deploy a persistent memosprite.
    let has_memo = path == "Remembrance";
    // is_fua: characters whose kit revolves around follow-up attacks.
    let is_fua = matches!(char.character_id.as_str(),
        "aventurine" | "clara" | "topaz" | "feixiao" | "march_7th_hunt" |
        "moze" | "dr_ratio" | "herta" | "himeko" | "jade" | "yunli" |
        "jing_yuan" | "lingsha" | "the_herta"
    );

    // ── Apply relic main stats (flat stats → base_stats; pct stats → buffs) ──
    // We build a temp member so we can call relics::apply_relic_main_stat.
    // HP and max_hp are finalised after set bonuses in run_simulation.
    let buffs = Buffs::default();
    // Element must be set before applying relic DMG% (element check inside).
    let element_clone = element.clone();
    let mut temp_member = TeamMember {
        kit_id:         char.character_id.clone(),
        name:           char.name.clone().unwrap_or_else(|| char.character_id.clone()),
        element:        element_clone,
        path:           path.clone(),
        level:          char.level.unwrap_or(80),
        eidolon:        char.eidolon.unwrap_or(0),
        hp:             0.0, // finalised later
        max_hp:         0.0, // finalised later
        shield:         0.0,
        is_downed:      false,
        toughness:      100.0,
        max_toughness:  100.0,
        is_broken:      false,
        energy:         0.0,
        max_energy:     120.0,
        ability_levels: AbilityLevels::default(),
        base_stats,
        buffs,
        active_buffs:   HashMap::new(),
        active_debuffs: HashMap::new(),
        lightcone:      LightconeStats { base_stats: lc_stats, scaling: 1.0, id: lc_id, superimposition: lc_si },
        stacks:         HashMap::new(),
        turn_counters:  HashMap::new(),
        aggro_modifier: 0.0,
        abilities,
        relics:         relic_list,
        has_memo,
        is_fua,
    };

    // Apply relic main stats now (flat additions go into base_stats, % into buffs)
    let relic_copy = temp_member.relics.clone();
    relics::apply_relics(&mut temp_member, &relic_copy);

    temp_member
}

fn map_wave(wave_data: &crate::models::IncomingWave) -> Wave {
    let mut initial: Vec<Option<SimEnemy>> = vec![None; 5];
    if let Some(enemies) = &wave_data.enemies {
        for (i, slot) in enemies.iter().enumerate() {
            if i >= 5 { break; }
            if let Some(e) = slot {
                let mut base_stats: HashMap<String, f64> = HashMap::new();
                if let Some(bs) = &e.basic_stats {
                    for (id, sv) in bs { base_stats.insert(id.clone(), sv.value); }
                }
                if let Some(adv) = &e.advanced_stats {
                    for (id, sv) in adv { base_stats.insert(id.clone(), sv.value); }
                }
                let hp       = base_stats.get(ids::ENEMY_HP_ID).copied().unwrap_or(10000.0);
                let toughness = base_stats.get(ids::ENEMY_TOUGHNESS_ID).copied().unwrap_or(100.0);

                let effect_res = base_stats.get(ids::ENEMY_EFFECT_RES_ID).copied().unwrap_or(0.0);
                initial[i] = Some(SimEnemy {
                    kit_id:       e.id.clone(),
                    instance_id:  e.instance_id.clone(),
                    name:         e.name.clone().unwrap_or_else(|| e.id.clone()),
                    level:        e.level.unwrap_or(80),
                    hp,
                    max_hp:       hp,
                    toughness,
                    max_toughness: toughness,
                    is_broken:    false,
                    weaknesses:   e.weaknesses.clone().unwrap_or_default(),
                    resistance:   0.2,
                    elemental_res: e.resistances.clone().unwrap_or_default(),
                    vulnerability: 0.0,
                    dmg_reduction: 0.0,
                    weaken:        0.0,
                    debuff_count:  0,
                    effect_res,
                    tier:          e.tier.clone().unwrap_or_else(|| "normal".to_string()),
                    active_debuffs: HashMap::new(),
                    active_buffs:   HashMap::new(),
                    base_stats,
                    cached_def_reduce: 0.0,
                    cached_all_res_reduce: 0.0,
                    cached_weakness_res_reduce: 0.0,
                    cached_vuln_bonus: 0.0,
                });
            }
        }
    }
    Wave { initial_enemies: initial, enemy_pool: Vec::new() }
}

// ─── Ability multiplier lookup ────────────────────────────────────────────────

fn get_ability_params(member: &TeamMember, action_type: &ActionType) -> (f64, String) {
    let idx = match action_type {
        ActionType::Basic     => 0,
        ActionType::Skill     => 1,
        ActionType::Ultimate  => 2,
        ActionType::TalentProc | ActionType::FollowUp => 3,
        ActionType::EnemyAttack => 0,
    };

    let target_level = match action_type {
        ActionType::Basic    => member.ability_levels.basic,
        ActionType::Skill    => member.ability_levels.skill,
        ActionType::Ultimate => member.ability_levels.ultimate,
        _                    => member.ability_levels.talent,
    };

    if let Some(ability) = member.abilities.get(idx) {
        if let Some(scalings) = &ability.scalings {
            let scaling = scalings.iter()
                .find(|s| s.level == target_level)
                .or_else(|| scalings.iter().max_by_key(|s| s.level));

            if let Some(s) = scaling {
                let stat_id = s.scaling_stat_id.clone()
                    .unwrap_or_else(|| ids::CHAR_ATK_ID.to_string());
                return (s.value / 100.0, stat_id);
            }
        }
    }

    (1.0, ids::CHAR_ATK_ID.to_string())
}

fn toughness_damage_for(action_type: &ActionType) -> f64 {
    match action_type {
        ActionType::Basic    => 10.0,
        ActionType::Skill    => 20.0,
        ActionType::Ultimate => 30.0,
        ActionType::FollowUp | ActionType::TalentProc => 10.0,
        ActionType::EnemyAttack => 0.0,
    }
}

// ─── Targeting ───────────────────────────────────────────────────────────────

fn pick_ally_target(state: &SimState) -> Option<usize> {
    let living: Vec<(usize, f64)> = state.team.iter().enumerate()
        .filter(|(_, m)| !m.is_downed)
        .map(|(i, m)| (i, base_aggro(&m.path) * (1.0 + m.aggro_modifier)))
        .collect();
    if living.is_empty() { return None; }
    living.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).map(|(i, _)| *i)
}

fn pick_enemy_target(state: &SimState) -> Option<usize> {
    state.enemies.iter().position(|e| e.as_ref().map_or(false, |e| e.hp > 0.0))
}

// ─── Damage application helpers ──────────────────────────────────────────────

/// Apply HP damage + toughness reduction for one action against one enemy slot.
/// Returns true if the enemy just broke.
pub fn apply_hit_to_enemy(
    state: &mut SimState,
    attacker_idx: usize,
    enemy_slot: usize,
    action: &ActionParams,
) -> bool {
    // Unnerved (Aventurine ult debuff): attacker gains +15% CRIT DMG vs this enemy
    let unnerved = state.enemies[enemy_slot].as_ref()
        .map_or(false, |e| e.active_debuffs.contains_key("aventurine_unnerved"));
    if unnerved {
        state.team[attacker_idx].buffs.crit_dmg += 15.0;
    }

    let (damage, maybe_components) = {
        let member = &state.team[attacker_idx];
        if let Some(enemy) = state.enemies[enemy_slot].as_ref() {
            if state.with_logs {
                let (dmg, comp) = damage::calculate_damage_detailed(member, enemy, action);
                (dmg, Some(comp))
            } else {
                (damage::calculate_damage(member, enemy, action), None)
            }
        } else {
            (0.0, None)
        }
    };

    if unnerved {
        state.team[attacker_idx].buffs.crit_dmg -= 15.0;
    }

    if damage <= 0.0 { return false; }

    let element          = state.team[attacker_idx].element.clone();
    let name             = state.team[attacker_idx].name.clone();
    let break_efficiency = state.team[attacker_idx].buffs.break_efficiency;

    // Build log message including target name
    let target_name = state.enemies[enemy_slot].as_ref()
        .map(|e| e.name.clone()).unwrap_or_default();
    let target_hp_before = state.enemies[enemy_slot].as_ref().map(|e| e.hp).unwrap_or(0.0);
    let target_max_hp    = state.enemies[enemy_slot].as_ref().map(|e| e.max_hp).unwrap_or(1.0);

    state.total_damage += damage;
    state.add_log(&name, format!("{:?} on {} -> {:.0} DMG", action.action_type, target_name, damage));

    // Damage formula breakdown (only when with_logs)
    if let Some(c) = maybe_components {
        state.add_log_sub(format!(
            "Stat:{:.0} ({:.0}+lc{:.0})x(1+{:.1}%) | Mult:{:.0}%x{:.0}={:.0} base",
            c.total_stat, c.char_base, c.lc_base,
            (c.total_stat / (c.char_base + c.lc_base + 1e-9) - 1.0) * 100.0,
            action.multiplier * 100.0, c.total_stat, c.base_dmg
        ));
        state.add_log_sub(format!(
            "Boost:x{:.3} | DEF:x{:.3} | RES:x{:.3} | Vuln:x{:.3} | Crit:x{:.3} | Broken:x{:.1}",
            c.dmg_boost, c.def_m, c.res_m, c.vuln_m, c.crit_m, c.broken_m
        ));
        let hp_after = (target_hp_before - damage).max(0.0);
        state.add_log_sub(format!(
            "Target HP: {:.0} -> {:.0} / {:.0} ({:.1}%)",
            target_hp_before, hp_after, target_max_hp,
            hp_after / target_max_hp * 100.0
        ));
    }

    let (is_weak, was_broken) = state.enemies[enemy_slot].as_ref()
        .map(|e| (e.weaknesses.contains(&element), e.is_broken))
        .unwrap_or((false, true));

    let toughness_dealt = if is_weak {
        calculate_toughness_dealt(break_efficiency, action.toughness_damage)
    } else {
        0.0
    };

    if let Some(e) = state.enemies[enemy_slot].as_mut() { e.hp -= damage; }

    let mut just_broke = false;
    if !was_broken && toughness_dealt > 0.0 {
        if let Some(e) = state.enemies[enemy_slot].as_mut() {
            e.toughness -= toughness_dealt;
            if e.toughness <= 0.0 {
                e.toughness = 0.0;
                e.is_broken = true;
                just_broke  = true;
            }
        }
        if just_broke {
            let break_dmg = {
                let member = &state.team[attacker_idx];
                state.enemies[enemy_slot].as_ref()
                    .map(|e| damage::calculate_break_damage(member, e))
                    .unwrap_or(0.0)
            };
            if break_dmg > 0.0 {
                if let Some(e) = state.enemies[enemy_slot].as_mut() { e.hp -= break_dmg; }
                state.total_damage += break_dmg;
                state.add_log(&name, format!("Break! {:.0} break DMG", break_dmg));
            }
        }
    }

    // Notify all team members that this enemy was just Weakness Broken.
    if just_broke {
        let n = state.team.len();
        for i in 0..n {
            characters::dispatch_on_break(state, i, enemy_slot);
        }
    }

    if state.enemies[enemy_slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[enemy_slot] = None;
        // Relic kill hooks (Sigonia stacks, etc.) — fire for all team members.
        relics::on_enemy_killed(&mut state.team);
    }

    just_broke
}

fn calculate_toughness_dealt(break_efficiency: f64, base: f64) -> f64 {
    base * (1.0 + break_efficiency / 100.0)
}

/// Generic single-target action damage (used for basic / skill / non-custom ults).
fn execute_action_damage(state: &mut SimState, idx: usize, action: &ActionParams, target_idx: Option<usize>) {
    if let Some(t_idx) = target_idx {
        apply_hit_to_enemy(state, idx, t_idx, action);
    }
}

/// Apply incoming damage to an ally, with DEF mitigation, shield absorption,
/// energy-from-hit, Bailu A6, and KO prevention.
pub fn apply_damage_to_ally(state: &mut SimState, target_idx: usize, raw_damage: f64) {
    // Arlan A6: nullify first direct hit
    if state.team[target_idx].stacks.get("arlan_a6_active").copied().unwrap_or(0.0) >= 1.0 {
        state.team[target_idx].stacks.remove("arlan_a6_active");
        return;
    }

    // ── DEF mitigation (HSR formula: DEF / (DEF + 200 + 10 × enemy_level)) ────
    // Enemy level defaults to 95 (endgame standard) for the generic case.
    let ally_def = {
        let m = &state.team[target_idx];
        let base_def = m.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(500.0)
            + m.lightcone.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(0.0);
        base_def * (1.0 + m.buffs.def_percent / 100.0)
    };
    let def_mit   = ally_def / (ally_def + 200.0 + 10.0 * 95.0);
    let dmg_red   = state.team[target_idx].buffs.incoming_dmg_reduction / 100.0;
    let damage    = raw_damage * (1.0 - def_mit) * (1.0 - dmg_red);

    // ── Energy from being hit (10 base, scaled by ERR) ───────────────────────
    let err_mult = 1.0 + state.team[target_idx].buffs.energy_regen_rate / 100.0;
    let max_energy = state.team[target_idx].max_energy;
    state.team[target_idx].energy = (state.team[target_idx].energy + 10.0 * err_mult)
        .min(max_energy);

    // ── Shield absorption ─────────────────────────────────────────────────────
    let shield      = state.team[target_idx].shield;
    let absorbed    = damage.min(shield);
    let remaining   = damage - absorbed;

    // Bailu A6: Invigorated allies take 10% less DMG
    let invigorated = state.team[target_idx].active_buffs.contains_key("bailu_invigoration");
    let reduced     = if invigorated { remaining * 0.9 } else { remaining };

    state.team[target_idx].shield -= absorbed;
    state.team[target_idx].hp    -= reduced;

    // Relic hit-taken hooks (Champion stack, Wavestrider stack).
    relics::on_hit_taken(&mut state.team[target_idx]);

    if state.team[target_idx].hp <= 0.0 {
        // Arlan E4: survive killing blow once
        if state.team[target_idx].stacks.get("arlan_e4_active").copied().unwrap_or(0.0) >= 1.0 {
            state.team[target_idx].stacks.remove("arlan_e4_active");
            let max_hp = state.team[target_idx].max_hp;
            state.team[target_idx].hp = max_hp * 0.25;
            return;
        }

        // Bailu talent: KO prevention
        let bailu_idx = state.team.iter()
            .position(|m| m.kit_id == ids::BAILU_ID && !m.is_downed);
        let has_revive = bailu_idx.map_or(false, |bi|
            state.team[bi].stacks.get("bailu_ko_revives").copied().unwrap_or(0.0) > 0.0
        );
        if has_revive {
            let bi = bailu_idx.unwrap();
            *state.team[bi].stacks.entry("bailu_ko_revives").or_insert(0.0) -= 1.0;
            let max_hp = state.team[target_idx].max_hp;
            state.team[target_idx].hp = max_hp * 0.01; // keep alive at ~1%
            return;
        }

        state.team[target_idx].hp = 0.0;
        state.team[target_idx].is_downed = true;
    }
}

// ─── Ult-readiness check ──────────────────────────────────────────────────────

fn ult_ready(state: &SimState, idx: usize) -> bool {
    let m = &state.team[idx];
    // Characters may signal readiness via "_ult_ready" stack
    if m.stacks.get("_ult_ready").copied().unwrap_or(0.0) >= 1.0 {
        return true;
    }
    // Acheron uses SD stacks (9 = ult threshold)
    if m.kit_id == ids::ACHERON_ID {
        return m.stacks.get("sd").copied().unwrap_or(0.0) >= 9.0;
    }
    m.energy >= m.max_energy
}

// ─── Execute a single ally turn ───────────────────────────────────────────────

fn execute_ally_turn(state: &mut SimState, idx: usize) {
    if state.team[idx].is_downed { return; }

    // Tick status effects
    effects::tick_buffs(&mut state.team[idx]);
    effects::tick_debuffs(&mut state.team[idx]);

    // Relic turn-start effects (Guard 4p heal, etc.)
    relics::apply_turn_start_effects(&mut state.team[idx]);

    // on_turn_start hook
    characters::dispatch_on_turn_start(state, idx);

    // Rich turn-start header
    log_turn_start(state, idx);
    log_enemy_snapshot(state);

    // Choose normal action: always Basic or Skill (ult fires separately after)
    let action_type = if state.skill_points > 0 {
        ActionType::Skill
    } else {
        ActionType::Basic
    };

    let (multiplier, scaling_stat_id) = get_ability_params(&state.team[idx], &action_type);
    let toughness_dmg = toughness_damage_for(&action_type);
    let mut action = ActionParams {
        action_type:      action_type.clone(),
        scaling_stat_id,
        multiplier,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: toughness_dmg,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let target_idx = pick_enemy_target(state);

    // Snapshot buffs: on_before_action may add per-action temporary boosts
    // (e.g. Acheron E1 +CR, SW A6 +ATK%, Aventurine E6 +DMG%) that must not
    // accumulate across turns — they are valid only for the current action.
    let buffs_snapshot = state.team[idx].buffs.clone();

    // on_before_action (may mutate action, buffs, or stacks)
    state.current_action_id += 1;
    characters::dispatch_on_before_action(state, idx, &mut action, target_idx);
    lightcones::dispatch_on_before_action(state, idx, &mut action, target_idx);

    // Relic combat conditionals (after character hooks, before damage).
    // Uses the live enemy state to apply set bonuses that depend on debuff count,
    // weakness presence, or accumulated stacks (Champion, Sigonia, etc.).
    {
        let target = target_idx.and_then(|t| state.enemies[t].as_ref());
        relics::apply_action_conditional_buffs(&mut state.team[idx], target, &action_type);
    }

    // SP and energy accounting (ERR multiplies all energy gains)
    let err_mult = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
    match action_type {
        ActionType::Basic => {
            state.skill_points = (state.skill_points + 1).min(5);
            state.team[idx].energy += 20.0 * err_mult;
        }
        ActionType::Skill => {
            state.skill_points -= 1;
            state.team[idx].energy += 30.0 * err_mult;
        }
        _ => {}
    }

    // Apply damage
    execute_action_damage(state, idx, &action, target_idx);

    // Record the attack for stack-based relic effects (Champion 4p, etc.).
    if target_idx.is_some() {
        relics::on_attack_hit(&mut state.team[idx]);
    }

    // on_after_action
    characters::dispatch_on_after_action(state, idx, &action, target_idx);
    lightcones::dispatch_on_after_action(state, idx, &action, target_idx);

    // on_global_debuff (fires for all debuff-inflicting actions)
    if action.inflicts_debuff {
        if let Some(t_idx) = target_idx {
            characters::dispatch_on_global_debuff(state, idx, t_idx);
        }
    }

    // Notify other team members of this action
    characters::dispatch_on_ally_action(state, idx, &action, target_idx);

    // Restore pre-action buffs: removes temporary boosts added by on_before_action.
    // Persistent state changes (stacks, active_buffs, energy, enemy HP) are unaffected.
    state.team[idx].buffs = buffs_snapshot;

    // Post-action relic hooks: set windows (Band Skill → ATK window) and team buffs.
    // Called after snapshot restore so persistent changes outlive the action.
    relics::on_action_used(&mut state.team, idx, &action_type);

    // ─── Fire ult if ready (TS: ult fires after the normal action on the same turn) ───
    if ult_ready(state, idx) {
        execute_ult(state, idx);
    }
}

// ─── Execute an ultimate ─────────────────────────────────────────────────────

fn execute_ult(state: &mut SimState, idx: usize) {
    // Clear the custom readiness flag if it was used
    state.team[idx].stacks.remove("_ult_ready");

    let target_idx = pick_enemy_target(state);
    let (multiplier, scaling_stat_id) = get_ability_params(&state.team[idx], &ActionType::Ultimate);
    let toughness_dmg = toughness_damage_for(&ActionType::Ultimate);

    let mut action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id,
        multiplier,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: toughness_dmg,
        inflicts_debuff:  true,   // ults always count as debuff-inflicting for Acheron
        is_ult_dmg:       true,
    };

    // Snapshot buffs before on_before_action (same reason as in execute_ally_turn)
    let buffs_snapshot = state.team[idx].buffs.clone();

    state.current_action_id += 1;
    characters::dispatch_on_before_action(state, idx, &mut action, target_idx);
    lightcones::dispatch_on_before_action(state, idx, &mut action, target_idx);

    // Relic combat conditionals for the ult action (Wavestrider stack consumption,
    // Hunter window, Sigonia stacks, Pioneer/Wastelander vs enemy debuffs, etc.).
    {
        let target = target_idx.and_then(|t| state.enemies[t].as_ref());
        relics::apply_action_conditional_buffs(&mut state.team[idx], target, &ActionType::Ultimate);
    }

    // Reset energy (energy-based characters); Acheron resets in on_ult
    if state.team[idx].kit_id != ids::ACHERON_ID {
        state.team[idx].energy = 0.0;
    }

    // on_ult: character may handle all damage internally and set "_ult_handled"
    characters::dispatch_on_ult(state, idx);

    // If character didn't handle ult damage, do generic single-target damage
    let handled = state.team[idx].stacks.remove("_ult_handled").is_some();
    if !handled {
        execute_action_damage(state, idx, &action, target_idx);
    }

    // Record the ult attack for stack-based relic effects (Champion 4p, etc.).
    if target_idx.is_some() {
        relics::on_attack_hit(&mut state.team[idx]);
    }

    // on_after_action
    characters::dispatch_on_after_action(state, idx, &action, target_idx);
    lightcones::dispatch_on_after_action(state, idx, &action, target_idx);

    // on_global_debuff for the ult
    if action.inflicts_debuff {
        if let Some(t_idx) = target_idx {
            characters::dispatch_on_global_debuff(state, idx, t_idx);
        }
    }

    // Restore buffs after ult (removes temporary action-scoped boosts)
    state.team[idx].buffs = buffs_snapshot;

    // Post-ult relic hooks: set windows (Hunter, Firesmith) and team buffs (Messenger, Watchmaker).
    // Called after snapshot restore so persistent changes outlive the ult.
    relics::on_action_used(&mut state.team, idx, &ActionType::Ultimate);
}

// ─── Execute a single enemy turn ─────────────────────────────────────────────

fn execute_enemy_turn(state: &mut SimState, enemy_instance_id: &str) {
    let e_idx = match state.find_enemy_idx(enemy_instance_id) {
        Some(i) => i,
        None    => return,
    };

    effects::tick_enemy_debuffs(state.enemies[e_idx].as_mut().unwrap());

    // Handle Freeze: skip turn
    if state.enemies[e_idx].as_ref().unwrap().active_debuffs.contains_key("Freeze") {
        return;
    }

    // on_enemy_turn_start for all allies
    characters::dispatch_on_enemy_turn_start(state, e_idx);
    enemies::dispatch_on_turn_start(state, e_idx);

    // Log enemy state at turn start
    if state.with_logs {
        let (e_name, e_hp, e_max_hp, e_tgh, e_max_tgh, debuff_str) = {
            let e = state.enemies[e_idx].as_ref().unwrap();
            let debuffs = if e.active_debuffs.is_empty() { "none".to_string() } else {
                e.active_debuffs.iter()
                    .map(|(k, v)| format!("{}({:.1},{}t)", k, v.value, v.duration))
                    .collect::<Vec<_>>().join(", ")
            };
            (e.name.clone(), e.hp, e.max_hp, e.toughness, e.max_toughness, debuffs)
        };
        state.add_log(&e_name, format!(
            "Turn | HP:{:.0}/{:.0} ({:.1}%) | TGH:{:.0}/{:.0}",
            e_hp, e_max_hp, e_hp / e_max_hp * 100.0, e_tgh, e_max_tgh
        ));
        state.add_log_sub(format!("Debuffs: {}", debuff_str));
    }

    // Enemy attack: try kit dispatch first, fall back to generic
    if let Some(ally_idx) = pick_ally_target(state) {
        let e_name        = state.enemies[e_idx].as_ref().unwrap().name.clone();
        let ally_hp_before = state.team[ally_idx].hp;
        let ally_name      = state.team[ally_idx].name.clone();
        let ally_max_hp    = state.team[ally_idx].max_hp;

        if let Some((damage, log)) = enemies::dispatch_on_action(state, e_idx, ally_idx) {
            if damage > 0.0 {
                apply_damage_to_ally(state, ally_idx, damage);
                state.add_log(&e_name, log);
                if state.with_logs {
                    let hp_after = state.team[ally_idx].hp.max(0.0);
                    state.add_log_sub(format!(
                        "{} HP: {:.0} -> {:.0} / {:.0} ({:.1}%)",
                        ally_name, ally_hp_before, hp_after, ally_max_hp,
                        hp_after / ally_max_hp * 100.0
                    ));
                }
            }
        } else {
            // Generic fallback: 80% ATK flat damage
            let damage = state.enemies[e_idx].as_ref()
                .map(|e| e.base_stats.get(ids::ENEMY_ATK_ID).copied().unwrap_or(1000.0) * 0.8)
                .unwrap_or(0.0);
            if damage > 0.0 {
                apply_damage_to_ally(state, ally_idx, damage);
                state.add_log(&e_name, format!("attacks for {:.0} DMG", damage));
                if state.with_logs {
                    let hp_after = state.team[ally_idx].hp.max(0.0);
                    state.add_log_sub(format!(
                        "{} HP: {:.0} -> {:.0} / {:.0} ({:.1}%)",
                        ally_name, ally_hp_before, hp_after, ally_max_hp,
                        hp_after / ally_max_hp * 100.0
                    ));
                }
            }
        }
    }

    // on_enemy_action for all allies
    characters::dispatch_on_enemy_action(state, e_idx);
}

// ─── Wave management ─────────────────────────────────────────────────────────

fn advance_wave(state: &mut SimState) {
    state.current_wave_index += 1;
    if state.current_wave_index >= state.waves.len() { return; }

    let next_wave = state.waves[state.current_wave_index].initial_enemies.clone();
    state.enemies = next_wave;

    for slot in &state.enemies {
        if let Some(enemy) = slot {
            let spd = enemy.base_stats.get(ids::ENEMY_SPD_ID).copied().unwrap_or(100.0);
            state.av_queue.push(ActorEntry {
                next_av:     state.current_av + 10000.0 / spd,
                actor_id:    enemy.kit_id.clone(),
                instance_id: enemy.instance_id.clone(),
                is_enemy:    true,
            });
        }
    }
}

// ─── Main simulation entry point ─────────────────────────────────────────────

/// Compute effective SPD for a team member (base × speed% multiplier).
/// Used everywhere the AV queue needs to re-schedule an actor.
/// Includes persistent relic SPD buffs that are tracked in stacks rather than the snapshot.
pub fn effective_spd(member: &TeamMember) -> f64 {
    let base = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    // buffs.speed_percent already includes persistent Messenger SPD (applied directly there).
    base * (1.0 + member.buffs.speed_percent / 100.0)
}

pub fn run_simulation(
    chars: &[IncomingCharacter],
    lcs: &[Option<&IncomingLightcone>],
    waves_data: &[crate::models::IncomingWave],
    max_cycles: i32,
    with_logs: bool,
) -> SimReport {
    // ── Step 1: build team members (relic main stats already applied inside) ──
    let mut team: Vec<TeamMember> = chars.iter().enumerate()
        .map(|(i, c)| map_to_team_member(c, lcs.get(i).and_then(|lc| *lc)))
        .collect();

    // ── Step 2: per-character relic + planar set bonuses ─────────────────────
    // Clone relic lists so we can borrow `team` mutably in the loop.
    let relic_lists: Vec<Vec<IncomingRelic>> = team.iter()
        .map(|m| m.relics.clone())
        .collect();
    for i in 0..team.len() {
        relics::apply_set_bonuses(&mut team[i], &relic_lists[i]);
        planars::apply_set_bonuses(&mut team[i], &relic_lists[i]);
    }

    // ── Step 3: team-wide conditional set bonuses ────────────────────────────
    relics::apply_team_set_bonuses(&mut team, &relic_lists);
    planars::apply_team_set_bonuses(&mut team, &relic_lists);

    // ── Step 3b: battle-start relic effects (Passerby 4p +SP, etc.) ─────────
    let bonus_sp = relics::apply_battle_start_effects(&team);

    // ── Step 4: finalise HP (apply hp_percent multiplier now that it's complete)
    for member in &mut team {
        let base_hp = member.base_stats.get(ids::CHAR_HP_ID).copied().unwrap_or(3000.0);
        let final_hp = (base_hp * (1.0 + member.buffs.hp_percent / 100.0)).floor();
        member.max_hp = final_hp;
        member.hp     = final_hp;
    }

    let nihility_count = team.iter().filter(|m| m.path == "Nihility").count() as i32;

    let waves: Vec<Wave> = waves_data.iter().map(map_wave).collect();
    let first_enemies = if waves.is_empty() {
        vec![]
    } else {
        waves[0].initial_enemies.clone()
    };

    let max_av = 150.0 + (max_cycles - 1) as f64 * 100.0;

    let mut state = SimState {
        team,
        enemies:           first_enemies,
        waves,
        current_wave_index: 0,
        av_queue:          BinaryHeap::new(),
        current_av:        0.0,
        max_av,
        skill_points:      (3 + bonus_sp).min(5),
        total_damage:      0.0,
        logs:              Vec::new(),
        nihility_count,
        with_logs,
        stacks:            HashMap::new(),
        current_action_id: 0,
    };

    // Init AV queue for allies (use effective SPD which includes speed_percent)
    for i in 0..state.team.len() {
        let spd = effective_spd(&state.team[i]);
        state.av_queue.push(ActorEntry {
            next_av:     10000.0 / spd,
            actor_id:    state.team[i].kit_id.clone(),
            instance_id: String::new(),
            is_enemy:    false,
        });
    }

    // Init AV queue for enemies
    for slot in &state.enemies {
        if let Some(enemy) = slot {
            let spd = enemy.base_stats.get(ids::ENEMY_SPD_ID).copied().unwrap_or(100.0);
            state.av_queue.push(ActorEntry {
                next_av:     10000.0 / spd,
                actor_id:    enemy.kit_id.clone(),
                instance_id: enemy.instance_id.clone(),
                is_enemy:    true,
            });
        }
    }

    // on_battle_start hooks (characters set max_energy, stat boosts, etc.)
    for i in 0..state.team.len() {
        characters::dispatch_on_battle_start(&mut state, i);
        lightcones::dispatch_on_battle_start(&mut state, i);
    }
    // enemy on_battle_start hooks
    for i in 0..state.enemies.len() {
        if state.enemies[i].is_some() {
            enemies::dispatch_on_battle_start(&mut state, i);
        }
    }

    // Re-evaluate Effect RES-gated bonuses (e.g. Broken Keel 4p) now that
    // minor traces and character talents have applied their Effect RES grants.
    planars::apply_effect_res_bonuses(&mut state.team, &relic_lists);

    // ─── Main combat loop ──────────────────────────────────────────────────────

    let mut is_defeated = false;

    loop {
        let entry = match state.av_queue.pop() {
            Some(e) => e,
            None    => break,
        };

        state.current_av = entry.next_av;

        if state.current_av > state.max_av { break; }

        if state.all_enemies_dead() {
            if state.current_wave_index + 1 >= state.waves.len() {
                break;
            } else {
                advance_wave(&mut state);
            }
        }

        if state.living_count() == 0 {
            is_defeated = true;
            break;
        }

        if entry.is_enemy {
            if state.find_enemy_idx(&entry.instance_id).is_none() { continue; }
            execute_enemy_turn(&mut state, &entry.instance_id);

            if let Some(e_idx) = state.find_enemy_idx(&entry.instance_id) {
                let spd = state.enemies[e_idx].as_ref()
                    .and_then(|e| e.base_stats.get(ids::ENEMY_SPD_ID).copied())
                    .unwrap_or(100.0);
                state.av_queue.push(ActorEntry {
                    next_av:     entry.next_av + 10000.0 / spd,
                    actor_id:    entry.actor_id,
                    instance_id: entry.instance_id,
                    is_enemy:    true,
                });
            }
        } else if entry.actor_id == ids::GARMENTMAKER_ID {
            // ── Garmentmaker memosprite turn ─────────────────────────────────
            // Validate generation: if Aglaea re-summoned Garmentmaker the old
            // entry's instance_id no longer matches garmentmaker_gen → drop it.
            let current_gen = state.stacks.get("garmentmaker_gen").copied().unwrap_or(0.0);
            if entry.instance_id != current_gen.to_string() {
                continue; // stale entry from a previous summon
            }

            let aglaea_idx = state.team.iter().position(|m| m.kit_id == ids::AGLAEA_ID);
            if let Some(a_idx) = aglaea_idx {
                let alive_and_present = !state.team[a_idx].is_downed
                    && characters::aglaea::is_gm_alive(&state);
                if alive_and_present {
                    characters::aglaea::garmentmaker_turn(&mut state, a_idx);

                    // Re-schedule only if Garmentmaker survived (countdown > 0).
                    if characters::aglaea::is_gm_alive(&state) {
                        let gm_spd = state.stacks.get("garmentmaker_spd").copied().unwrap_or(100.0);
                        state.av_queue.push(ActorEntry {
                            next_av:     entry.next_av + 10000.0 / gm_spd,
                            actor_id:    entry.actor_id,
                            instance_id: entry.instance_id,
                            is_enemy:    false,
                        });
                    }
                }
            }
        } else {
            let char_idx = match state.team.iter().position(|m| m.kit_id == entry.actor_id) {
                Some(i) => i,
                None    => continue,
            };

            if state.team[char_idx].is_downed { continue; }

            execute_ally_turn(&mut state, char_idx);

            let spd = effective_spd(&state.team[char_idx]);
            state.av_queue.push(ActorEntry {
                next_av:     entry.next_av + 10000.0 / spd,
                actor_id:    entry.actor_id,
                instance_id: entry.instance_id,
                is_enemy:    false,
            });
        }
    }

    let cycles_taken = ((state.current_av - 150.0) / 100.0).ceil().max(0.0) as i32 + 1;

    SimReport {
        total_damage:  state.total_damage,
        cycles_taken:  cycles_taken.min(max_cycles),
        logs:          state.logs,
        is_defeated,
    }
}
