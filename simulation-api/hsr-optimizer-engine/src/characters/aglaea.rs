use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, ActorEntry, SimState, StatusEffect};

// ─── Custom energy tracking ────────────────────────────────────────────────────
const ENERGY_KEY:       &str = "aglaea_energy";
const STANCE_KEY:       &str = "aglaea_stance";
const ENERGY_CAP:       f64  = 350.0;

// ─── Garmentmaker state keys ──────────────────────────────────────────────────
const GM_ALIVE_KEY:     &str = "garmentmaker_alive";
const GM_COUNTDOWN_KEY: &str = "garmentmaker_countdown";
const GM_SPD_KEY:       &str = "garmentmaker_spd";
const GM_MAX_HP_KEY:    &str = "garmentmaker_max_hp";
const GM_HP_KEY:        &str = "garmentmaker_hp";
const GM_GEN_KEY:       &str = "garmentmaker_gen";
const GM_SPD_BOOST_KEY: &str = "gm_spd_boost_stacks";
const GM_RETAINED_KEY:  &str = "gm_retained_spd_boost"; // A4
const E2_STACKS_KEY:    &str = "agl_e2_stacks";

const GM_COUNTDOWN_MAX: f64 = 3.0;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn add_energy(state: &mut SimState, idx: usize, amount: f64) {
    let cur = state.stacks.get(ENERGY_KEY).copied().unwrap_or(0.0);
    state.stacks.insert(ENERGY_KEY.to_string(), (cur + amount).min(ENERGY_CAP));
    if state.stacks.get(ENERGY_KEY).copied().unwrap_or(0.0) >= ENERGY_CAP {
        state.team[idx].stacks.insert("_ult_ready".to_string(), 1.0);
    }
}

fn in_stance(state: &SimState) -> bool {
    state.stacks.get(STANCE_KEY).copied().unwrap_or(0.0) >= 1.0
}

pub fn is_gm_alive(state: &SimState) -> bool {
    state.stacks.get(GM_ALIVE_KEY).copied().unwrap_or(0.0) >= 1.0
}

fn gm_spd_boost_stacks(state: &SimState) -> f64 {
    state.stacks.get(GM_SPD_BOOST_KEY).copied().unwrap_or(0.0)
}

fn max_spd_boost_stacks(eidolon: i32) -> f64 {
    if eidolon >= 4 { 7.0 } else { 6.0 }
}

fn gm_spd_for(state: &SimState, aglaea_idx: usize) -> f64 {
    let base = state.team[aglaea_idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    let eff  = base * (1.0 + state.team[aglaea_idx].buffs.speed_percent / 100.0);
    eff * 0.35
}

/// Aglaea's effective SPD, adding SPD Boost stacks during Supreme Stance (+55 each).
fn aglaea_eff_spd(state: &SimState, idx: usize) -> f64 {
    let base = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    let eff  = base * (1.0 + state.team[idx].buffs.speed_percent / 100.0);
    if in_stance(state) { eff + gm_spd_boost_stacks(state) * 55.0 } else { eff }
}

/// Dismiss Garmentmaker, firing A4 (retain 1 stack) and Bloom of Drying Grass (+20 Energy).
fn dismiss_garmentmaker(state: &mut SimState, aglaea_idx: usize) {
    state.stacks.insert(GM_ALIVE_KEY.to_string(), 0.0);

    // Clear Seam Stitch marks — they are tied to GM's presence.
    for slot in state.enemies.iter_mut() {
        if let Some(e) = slot.as_mut() {
            e.active_buffs.remove("seam_stitch");
            e.active_buffs.remove("seam_stitch_vuln");
        }
    }

    // A4 "Last Thread of Fate": retain up to 1 SPD Boost stack for next summon.
    let stacks   = gm_spd_boost_stacks(state);
    let retained = stacks.min(1.0);
    state.stacks.insert(GM_RETAINED_KEY.to_string(),  retained);
    state.stacks.insert(GM_SPD_BOOST_KEY.to_string(), 0.0);

    // "Bloom of Drying Grass": Aglaea gains 20 Energy when GM disappears.
    add_energy(state, aglaea_idx, 20.0);

    let name = state.team[aglaea_idx].name.clone();
    state.add_log(&name, format!(
        "Garmentmaker disappears — +20 Energy (A4: {:.0} SPD Boost stack retained)", retained
    ));
}

/// Summon or re-summon Garmentmaker.
/// `advance_action = true` → "The Speeding Summer": GM fires immediately (AV = current).
fn summon_garmentmaker(state: &mut SimState, aglaea_idx: usize, advance_action: bool) {
    let gen       = state.stacks.get(GM_GEN_KEY).copied().unwrap_or(0.0) + 1.0;
    let gm_spd    = gm_spd_for(state, aglaea_idx);
    let gm_max_hp = 0.66 * state.team[aglaea_idx].max_hp + 720.0;

    // A4: inherit retained SPD Boost stack.
    let eidolon   = state.team[aglaea_idx].eidolon;
    let retained  = state.stacks.get(GM_RETAINED_KEY).copied().unwrap_or(0.0);
    let new_stacks = retained.min(max_spd_boost_stacks(eidolon));
    state.stacks.insert(GM_SPD_BOOST_KEY.to_string(), new_stacks);
    state.stacks.insert(GM_RETAINED_KEY.to_string(),  0.0);

    state.stacks.insert(GM_GEN_KEY.to_string(),       gen);
    state.stacks.insert(GM_ALIVE_KEY.to_string(),     1.0);
    state.stacks.insert(GM_COUNTDOWN_KEY.to_string(), GM_COUNTDOWN_MAX);
    state.stacks.insert(GM_SPD_KEY.to_string(),       gm_spd);
    state.stacks.insert(GM_MAX_HP_KEY.to_string(),    gm_max_hp);
    state.stacks.insert(GM_HP_KEY.to_string(),         gm_max_hp);

    // "The Speeding Summer": 100% action advance on summon → first turn at current AV.
    let first_av = if advance_action { state.current_av } else { state.current_av + 10000.0 / gm_spd };

    state.av_queue.push(ActorEntry {
        next_av:     first_av,
        actor_id:    ids::GARMENTMAKER_ID.to_string(),
        instance_id: gen.to_string(),
        is_enemy:    false,
    });

    let name = state.team[aglaea_idx].name.clone();
    state.add_log(&name, format!(
        "Garmentmaker summoned — SPD:{:.1} HP:{:.0} SPD Boost:{:.0}{}",
        gm_spd, gm_max_hp, new_stacks,
        if advance_action { " [immediate action]" } else { "" }
    ));
}

/// Apply splash damage to slots adjacent to `t` using the given action params.
fn apply_adjacent_hits(
    state:  &mut SimState,
    member: &crate::models::TeamMember,
    t:      usize,
    action: &ActionParams,
) -> f64 {
    let len = state.enemies.len();
    let adj_slots: Vec<usize> = [
        if t > 0 { Some(t - 1) } else { None },
        if t + 1 < len { Some(t + 1) } else { None },
    ]
    .into_iter().flatten().collect();

    let mut total = 0.0f64;
    for &adj in &adj_slots {
        if state.enemies[adj].as_ref().map_or(false, |e| e.hp > 0.0) {
            let dmg = state.enemies[adj].as_ref()
                .map(|e| damage::calculate_damage(member, e, action))
                .unwrap_or(0.0);
            if dmg > 0.0 {
                if let Some(e) = state.enemies[adj].as_mut() { e.hp -= dmg; }
                total += dmg;
            }
            if state.enemies[adj].as_ref().map_or(false, |e| e.hp <= 0.0) {
                state.enemies[adj] = None;
            }
        }
    }
    total
}

fn apply_seam_stitch_mark(state: &mut SimState, t: usize, eidolon: i32) {
    if let Some(e) = state.enemies[t].as_mut() {
        effects::apply_enemy_buff(e, "seam_stitch", StatusEffect {
            duration: 999,
            value:    1.0,
            stat:     None,
            effects:  vec![],
        });
        // E1: Seam Stitch enemies take +15% incoming DMG (vulnerability-like)
        if eidolon >= 1 {
            effects::apply_enemy_buff(e, "seam_stitch_vuln", StatusEffect {
                duration: 999,
                value:    15.0,
                stat:     Some("Vulnerability".to_string()),
                effects:  vec![],
            });
        }
    }
}

// ─── Character hooks ──────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy        = f64::MAX; // custom energy via stacks
    state.team[idx].buffs.crit_rate   += 12.0;   // minor trace: CRIT Rate +12%
    state.team[idx].buffs.dmg_boost   += 22.4;   // minor trace: Lightning DMG +22.4%
    state.team[idx].buffs.def_percent += 12.5;   // minor trace: DEF +12.5%

    state.stacks.insert(ENERGY_KEY.to_string(),     175.0); // A6: start with 175 energy
    state.stacks.insert(STANCE_KEY.to_string(),       0.0);
    state.stacks.insert(GM_ALIVE_KEY.to_string(),     0.0);
    state.stacks.insert(GM_GEN_KEY.to_string(),       0.0);
    state.stacks.insert(GM_SPD_BOOST_KEY.to_string(), 0.0);
    state.stacks.insert(GM_RETAINED_KEY.to_string(),  0.0);
    state.stacks.insert(E2_STACKS_KEY.to_string(),    0.0);
}

pub fn on_turn_start(_state: &mut SimState, _idx: usize) {}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    let eidolon = state.team[idx].eidolon;

    // E2 "Sail on the Raft of Eyelids": +14% DEF ignore per consecutive Aglaea/GM action (max 3).
    // Stacks reset when any unit other than Aglaea or GM acts (see on_ally_action).
    if eidolon >= 2 {
        let stacks = (state.stacks.get(E2_STACKS_KEY).copied().unwrap_or(0.0) + 1.0).min(3.0);
        state.stacks.insert(E2_STACKS_KEY.to_string(), stacks);
        state.team[idx].buffs.def_ignore += stacks * 14.0;
    }

    // A2 "The Myopic's Doom": in Supreme Stance, ATK += 720% × Aglaea_SPD + 360% × GM_SPD (flat).
    // Applied only for Basic (Enhanced Basic) — the sole damaging action in Stance.
    if in_stance(state) && action.action_type == ActionType::Basic {
        let agl_spd   = aglaea_eff_spd(state, idx);
        let gm_spd    = state.stacks.get(GM_SPD_KEY).copied().unwrap_or(0.0);
        let atk_bonus = agl_spd * 7.20 + gm_spd * 3.60;
        let cur_atk   = state.team[idx].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
        state.team[idx].base_stats.insert(ids::CHAR_ATK_ID.to_string(), cur_atk + atk_bonus);
        // Tag it so we can undo it after the damage clone in on_after_action.
        state.stacks.insert("agl_a2_atk_bonus".to_string(), atk_bonus);
    }

    // E6 "Fluctuate in the Tapestry of Fates": +20% Lightning RES PEN in Supreme Stance.
    if eidolon >= 6 && in_stance(state) {
        state.team[idx].buffs.res_pen += 20.0;
    }

    // E6: Joint ATK DMG bonus scaling with effective SPD (160/240/320 thresholds).
    if eidolon >= 6 && in_stance(state) && action.action_type == ActionType::Basic {
        let eff_spd   = aglaea_eff_spd(state, idx);
        let spd_bonus = if eff_spd >= 320.0 { 60.0 }
                        else if eff_spd >= 240.0 { 30.0 }
                        else if eff_spd >= 160.0 { 10.0 }
                        else { 0.0 };
        state.team[idx].buffs.dmg_boost += spd_bonus;
    }

    // Supreme Stance: zero out the base Basic hit; Enhanced Basic fires in on_after_action.
    if in_stance(state) && action.action_type == ActionType::Basic {
        action.multiplier       = 0.0;
        action.toughness_damage = 0.0;
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            let eidolon  = state.team[idx].eidolon;
            let gm_alive = is_gm_alive(state);

            if let Some(t) = target_idx {
                // Clone member NOW — A2 flat ATK bonus is still live in base_stats.
                let member = state.team[idx].clone();

                if in_stance(state) && gm_alive {
                    // "Slash by a Thousandfold Kiss" (Enhanced Basic):
                    // Aglaea: 200% main + 90% adj.  Garmentmaker: 200% main + 90% adj.
                    // Both use Aglaea's stats. Combined: 400% main, 180% adj per slot.
                    let main_action = ActionParams {
                        action_type:      ActionType::Basic,
                        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                        multiplier:       2.0,
                        extra_multiplier: 0.0,
                        extra_dmg:        0.0,
                        toughness_damage: 20.0,
                        inflicts_debuff:  false,
                        is_ult_dmg:       false,
                    };
                    // Combined adj action (Aglaea 90% + GM 90% = 180%) applied in one pass.
                    let adj_action = ActionParams {
                        multiplier:       1.80,
                        toughness_damage: 10.0,
                        ..main_action.clone()
                    };

                    let had_seam = state.enemies[t].as_ref()
                        .map_or(false, |e| e.active_buffs.contains_key("seam_stitch"));

                    // Aglaea 200% + GM 200% on primary target.
                    let agl_main = state.enemies[t].as_ref()
                        .map(|e| damage::calculate_damage(&member, e, &main_action)).unwrap_or(0.0);
                    let gm_main  = state.enemies[t].as_ref()
                        .map(|e| damage::calculate_damage(&member, e, &main_action)).unwrap_or(0.0);
                    let main_dmg = agl_main + gm_main;
                    if main_dmg > 0.0 {
                        if let Some(e) = state.enemies[t].as_mut() { e.hp -= main_dmg; }
                        state.total_damage += main_dmg;
                    }

                    // Blast splash on adjacent slots (combined 180%).
                    let adj_dmg = apply_adjacent_hits(state, &member, t, &adj_action);
                    state.total_damage += adj_dmg;

                    if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
                        state.enemies[t] = None;
                    }

                    let name = state.team[idx].name.clone();
                    state.add_log(&name, format!(
                        "Enhanced Basic (Stance): {:.0} main + {:.0} adj DMG", main_dmg, adj_dmg
                    ));

                    // E1: after hitting a Seam Stitch target, +20 Energy for Aglaea.
                    if eidolon >= 1 && had_seam {
                        add_energy(state, idx, 20.0);
                    }

                    // E4: after Aglaea attacks, GM gains 1 SPD Boost stack.
                    if eidolon >= 4 {
                        let max_s = max_spd_boost_stacks(eidolon);
                        let cur   = gm_spd_boost_stacks(state);
                        if cur < max_s {
                            state.stacks.insert(GM_SPD_BOOST_KEY.to_string(), cur + 1.0);
                        }
                    }

                } else if gm_alive {
                    // Normal mode (no Stance) + GM alive: Talent proc — Seam Stitch hit (30% ATK).
                    let seam_action = ActionParams {
                        action_type:      ActionType::Basic,
                        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                        multiplier:       0.30,
                        extra_multiplier: 0.0,
                        extra_dmg:        0.0,
                        toughness_damage: 0.0,
                        inflicts_debuff:  false,
                        is_ult_dmg:       false,
                    };
                    let seam_dmg = state.enemies[t].as_ref()
                        .map(|e| damage::calculate_damage(&member, e, &seam_action))
                        .unwrap_or(0.0);
                    if seam_dmg > 0.0 {
                        if let Some(e) = state.enemies[t].as_mut() { e.hp -= seam_dmg; }
                        state.total_damage += seam_dmg;
                    }
                    if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
                        state.enemies[t] = None;
                    }

                    // Apply Seam Stitch mark (and E1 vulnerability) to the target.
                    apply_seam_stitch_mark(state, t, eidolon);

                    // E4: after Aglaea attacks, GM gains 1 SPD Boost stack.
                    if eidolon >= 4 {
                        let max_s = max_spd_boost_stacks(eidolon);
                        let cur   = gm_spd_boost_stacks(state);
                        if cur < max_s {
                            state.stacks.insert(GM_SPD_BOOST_KEY.to_string(), cur + 1.0);
                        }
                    }
                }
                // GM not alive → no Seam Stitch, no joint attack.
            }

            // Remove A2 flat ATK bonus (must happen after the member clone above used it).
            if let Some(bonus) = state.stacks.remove("agl_a2_atk_bonus") {
                let cur = state.team[idx].base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
                state.team[idx].base_stats.insert(ids::CHAR_ATK_ID.to_string(), cur - bonus);
            }

            add_energy(state, idx, 20.0);
            state.team[idx].energy = 0.0; // suppress standard energy double-count
        }

        ActionType::Skill => {
            add_energy(state, idx, 20.0);
            state.team[idx].energy = 0.0;

            if is_gm_alive(state) {
                // "Rise, Exalted Renown" (GM present): restore 50% of GM's Max HP.
                let gm_max_hp = state.stacks.get(GM_MAX_HP_KEY).copied().unwrap_or(0.0);
                let gm_hp     = state.stacks.get(GM_HP_KEY).copied().unwrap_or(0.0);
                let heal      = (gm_max_hp * 0.50).min(gm_max_hp - gm_hp);
                state.stacks.insert(GM_HP_KEY.to_string(), (gm_hp + heal).min(gm_max_hp));
                let name = state.team[idx].name.clone();
                state.add_log(&name, format!("Skill: Garmentmaker HP +{:.0}", heal));
            } else {
                // "Rise, Exalted Renown" (GM absent): summon with immediate action.
                summon_garmentmaker(state, idx, true);
            }
        }

        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].stacks.remove("_ult_ready");
    state.stacks.insert(ENERGY_KEY.to_string(), 5.0);

    // "Dance, Destined Weaveress": activate Supreme Stance.
    state.stacks.insert(STANCE_KEY.to_string(), 1.0);

    if is_gm_alive(state) {
        // GM already present: refill HP to max.
        let gm_max_hp = state.stacks.get(GM_MAX_HP_KEY).copied().unwrap_or(0.0);
        state.stacks.insert(GM_HP_KEY.to_string(), gm_max_hp);
    } else {
        // GM absent: summon with immediate action.
        summon_garmentmaker(state, idx, true);
    }

    // SPD boost: +15% of base SPD on first Stance entry (guard against compounding).
    if state.team[idx].stacks.get("aglaea_spd_boosted").copied().unwrap_or(0.0) < 1.0 {
        state.team[idx].stacks.insert("aglaea_spd_boosted".to_string(), 1.0);
        let cur_spd = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), cur_spd * 1.15);
        // Update GM's SPD to reflect Aglaea's new base (re-summon invalidates stale AV entry).
        if is_gm_alive(state) {
            let gen     = state.stacks.get(GM_GEN_KEY).copied().unwrap_or(0.0) + 1.0;
            let gm_spd  = gm_spd_for(state, idx);
            state.stacks.insert(GM_GEN_KEY.to_string(), gen);
            state.stacks.insert(GM_SPD_KEY.to_string(), gm_spd);
            state.av_queue.push(ActorEntry {
                next_av:     state.current_av + 10000.0 / gm_spd,
                actor_id:    ids::GARMENTMAKER_ID.to_string(),
                instance_id: gen.to_string(),
                is_enemy:    false,
            });
        }
    }

    let stacks = gm_spd_boost_stacks(state);
    let name   = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Supreme Stance active — SPD Boost stacks:{:.0} (+{:.0} SPD)",
        stacks, stacks * 55.0
    ));
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
    // E2: reset stacks when any unit OTHER than Aglaea or Garmentmaker acts.
    // dispatch_on_ally_action already skips source_idx == idx, and Garmentmaker
    // never calls dispatch_on_ally_action, so we can reset unconditionally here.
    if state.team[idx].eidolon >= 2 {
        state.stacks.insert(E2_STACKS_KEY.to_string(), 0.0);
    }
}

// ─── Garmentmaker turn (called directly by simulator.rs) ──────────────────────

pub fn garmentmaker_turn(state: &mut SimState, aglaea_idx: usize) {
    if !is_gm_alive(state) { return; }

    let countdown     = state.stacks.get(GM_COUNTDOWN_KEY).copied().unwrap_or(0.0);
    let new_countdown = (countdown - 1.0).max(0.0);
    state.stacks.insert(GM_COUNTDOWN_KEY.to_string(), new_countdown);

    let eidolon = state.team[aglaea_idx].eidolon;

    // E2: GM action counts as an Aglaea/GM action → increment stacks.
    if eidolon >= 2 {
        let stacks = (state.stacks.get(E2_STACKS_KEY).copied().unwrap_or(0.0) + 1.0).min(3.0);
        state.stacks.insert(E2_STACKS_KEY.to_string(), stacks);
    }

    // Pick primary target, preferring enemies marked with Seam Stitch.
    let primary_target = state.enemies.iter().enumerate()
        .filter(|(_, s)| s.as_ref().map_or(false, |e| e.hp > 0.0))
        .max_by_key(|(_, s)| s.as_ref().map_or(0u32, |e| {
            if e.active_buffs.contains_key("seam_stitch") { 1 } else { 0 }
        }))
        .map(|(i, _)| i);

    let member = state.team[aglaea_idx].clone();

    let Some(t) = primary_target else {
        if new_countdown <= 0.0 { dismiss_garmentmaker(state, aglaea_idx); }
        return;
    };

    // Check Seam Stitch BEFORE the hit (determines SPD Boost stack gain).
    let had_seam = state.enemies[t].as_ref()
        .map_or(false, |e| e.active_buffs.contains_key("seam_stitch"));

    // "Thorned Snare" (Memosprite Skill): Blast — 110% ATK primary + 66% ATK adjacent.
    let main_action = ActionParams {
        action_type:      ActionType::TalentProc,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       1.10,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 10.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };
    let adj_action = ActionParams { multiplier: 0.66, toughness_damage: 5.0, ..main_action.clone() };

    let main_dmg = state.enemies[t].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &main_action)).unwrap_or(0.0);
    if main_dmg > 0.0 {
        if let Some(e) = state.enemies[t].as_mut() { e.hp -= main_dmg; }
    }

    let adj_dmg = apply_adjacent_hits(state, &member, t, &adj_action);
    let total   = main_dmg + adj_dmg;
    state.total_damage += total;

    // Apply / refresh Seam Stitch mark to primary target.
    apply_seam_stitch_mark(state, t, eidolon);

    if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[t] = None;
    }

    // "A Body Brewed by Tears": hitting a Seam Stitch enemy → +1 SPD Boost stack (max 6/7).
    if had_seam {
        let max_s = max_spd_boost_stacks(eidolon);
        let cur   = gm_spd_boost_stacks(state);
        if cur < max_s {
            state.stacks.insert(GM_SPD_BOOST_KEY.to_string(), cur + 1.0);
        }
    }

    // E1 "Drift at the Whim of Venus": hitting a Seam Stitch target → +20 Energy for Aglaea.
    if eidolon >= 1 && had_seam {
        add_energy(state, aglaea_idx, 20.0);
    }

    let name = member.name.clone();
    state.add_log(&name, format!(
        "Garmentmaker Thorned Snare (countdown {}→{}): {:.0} main + {:.0} adj | SPD Boost: {:.0}",
        countdown as i32, new_countdown as i32, main_dmg, adj_dmg,
        gm_spd_boost_stacks(state)
    ));

    if new_countdown <= 0.0 {
        dismiss_garmentmaker(state, aglaea_idx);
    }
}
