//! Relic set bonus dispatch and main-stat application.
//!
//! # Main stat keys (sent in `IncomingRelic.main_stat`)
//!
//! Head:         "flat_hp"
//! Hands:        "flat_atk"
//! Body:         "hp_percent" | "atk_percent" | "def_percent"
//!               "crit_rate" | "crit_dmg" | "outgoing_healing" | "effect_hit_rate"
//! Feet:         "hp_percent" | "atk_percent" | "def_percent" | "speed"
//! Planar Sphere:"hp_percent" | "atk_percent" | "def_percent"
//!               "physical_dmg" | "fire_dmg" | "ice_dmg" | "lightning_dmg"
//!               "wind_dmg" | "quantum_dmg" | "imaginary_dmg"
//! Link Rope:    "hp_percent" | "atk_percent" | "def_percent"
//!               "break_effect" | "err"
//!
//! Adding a new relic set:
//!   1. Create `src/relics/<name>.rs` with `pub fn apply(member, count)`.
//!   2. Add `mod <name>;` below.
//!   3. Add a match arm in `apply_set_bonuses`.
//!   4. If it has a team bonus, add `pub fn apply_team` to the file and call it
//!      in `apply_team_set_bonuses`.

#![allow(dead_code)]

mod band_of_sizzling_thunder;
mod champion_of_streetwise_boxing;
mod diviner_of_distant_reach;
mod eagle_of_twilight_line;
mod ever_glorious_magical_girl;
mod firesmith_of_lava_forging;
mod genius_of_brilliant_stars;
mod guard_of_wuthering_snow;
mod hero_of_triumphant_song;
mod hunter_of_glacial_forest;
mod iron_cavalry_against_the_scourge;
mod knight_of_purity_palace;
mod longevous_disciple;
mod messenger_traversing_hackerspace;
mod musketeer;
mod passerby_of_wandering_cloud;
mod pioneer;
mod poet_of_mourning_collapse;
mod prisoner_in_deep_confinement;
mod sacerdos_relived_ordeal;
mod scholar_lost_in_erudition;
mod self_enshrouded_recluse;
mod the_ashblazing_grand_duke;
mod the_wind_soaring_valorous;
mod thief_of_shooting_meteor;
mod warrior_goddess_of_sun_and_thunder;
mod wastelander_of_banditry_desert;
mod watchmaker_master_of_dream_machinations;
mod wavestrider_captain;
mod world_remaking_deliverer;

use crate::ids;
use crate::models::{ActionType, IncomingRelic, RelicSlot, SimEnemy, TeamMember};

// ─── Main stat values (5★ max level) ─────────────────────────────────────────

/// Returns the numeric value for a given (slot, main_stat) at 5★ max level.
/// Returns 0.0 for invalid combinations so callers can skip.
pub fn main_stat_value(slot: &RelicSlot, stat: &str) -> f64 {
    match slot {
        RelicSlot::Head => match stat {
            "flat_hp"  => 705.0,
            _          => 0.0,
        },
        RelicSlot::Hands => match stat {
            "flat_atk" => 352.0,
            _          => 0.0,
        },
        RelicSlot::Body => match stat {
            "hp_percent"       => 43.2,
            "atk_percent"      => 43.2,
            "def_percent"      => 54.0,
            "crit_rate"        => 32.4,
            "crit_dmg"         => 64.8,
            "outgoing_healing" => 34.5,
            "effect_hit_rate"  => 43.2,
            _                  => 0.0,
        },
        RelicSlot::Feet => match stat {
            "hp_percent"  => 43.2,
            "atk_percent" => 43.2,
            "def_percent" => 54.0,
            "speed"       => 25.0,
            _             => 0.0,
        },
        RelicSlot::PlanarSphere => match stat {
            "hp_percent"      => 43.2,
            "atk_percent"     => 43.2,
            "def_percent"     => 54.0,
            "physical_dmg"
            | "fire_dmg"
            | "ice_dmg"
            | "lightning_dmg"
            | "wind_dmg"
            | "quantum_dmg"
            | "imaginary_dmg" => 38.8,
            _                 => 0.0,
        },
        RelicSlot::LinkRope => match stat {
            "hp_percent"    => 43.2,
            "atk_percent"   => 43.2,
            "def_percent"   => 54.0,
            "break_effect"  => 64.8,
            "err"           => 19.4,   // Energy Regeneration Rate
            _               => 0.0,
        },
    }
}

// ─── Main stat application ────────────────────────────────────────────────────

/// Apply the main stat of a single relic piece to a `TeamMember`.
///
/// Flat stats go directly into `base_stats` (so they participate in HP/ATK totals).
/// Percentage stats go into `buffs` fields.
/// Element DMG% only applies when the piece's element matches the character's element.
pub fn apply_relic_main_stat(member: &mut TeamMember, relic: &IncomingRelic) {
    let v = main_stat_value(&relic.slot, &relic.main_stat);
    if v == 0.0 { return; }

    match relic.main_stat.as_str() {
        // ── Flat additions → base_stats ──────────────────────────────────────
        "flat_hp" => {
            *member.base_stats.entry(ids::CHAR_HP_ID.to_string()).or_insert(0.0) += v;
        }
        "flat_atk" => {
            *member.base_stats.entry(ids::CHAR_ATK_ID.to_string()).or_insert(0.0) += v;
        }
        "speed" => {
            *member.base_stats.entry(ids::CHAR_SPD_ID.to_string()).or_insert(0.0) += v;
        }
        "break_effect" => {
            *member.base_stats.entry(ids::CHAR_BE_ID.to_string()).or_insert(0.0) += v;
        }

        // ── Percentage bonuses → buffs ────────────────────────────────────────
        "hp_percent"       => member.buffs.hp_percent       += v,
        "atk_percent"      => member.buffs.atk_percent      += v,
        "def_percent"      => member.buffs.def_percent      += v,
        "crit_rate"        => member.buffs.crit_rate        += v,
        "crit_dmg"         => member.buffs.crit_dmg         += v,
        "outgoing_healing" => member.buffs.outgoing_healing += v,
        "effect_hit_rate"  => member.buffs.effect_hit_rate  += v,
        "err"              => member.buffs.energy_regen_rate += v,

        // ── Element DMG% → dmg_boost only if element matches ─────────────────
        "physical_dmg"   => { if member.element == "Physical"  { member.buffs.dmg_boost += v; } }
        "fire_dmg"       => { if member.element == "Fire"       { member.buffs.dmg_boost += v; } }
        "ice_dmg"        => { if member.element == "Ice"        { member.buffs.dmg_boost += v; } }
        "lightning_dmg"  => { if member.element == "Lightning"  { member.buffs.dmg_boost += v; } }
        "wind_dmg"       => { if member.element == "Wind"       { member.buffs.dmg_boost += v; } }
        "quantum_dmg"    => { if member.element == "Quantum"    { member.buffs.dmg_boost += v; } }
        "imaginary_dmg"  => { if member.element == "Imaginary"  { member.buffs.dmg_boost += v; } }

        _ => {} // unknown stat — ignore
    }
}

/// Apply all relic main stats for a character.
pub fn apply_relics(member: &mut TeamMember, relics: &[IncomingRelic]) {
    for relic in relics {
        apply_relic_main_stat(member, relic);
    }
}

// ─── Set bonuses ─────────────────────────────────────────────────────────────

fn count_set(relics: &[IncomingRelic], set_id: &str) -> usize {
    relics.iter().filter(|r| r.set_id == set_id).count()
}

/// Apply per-character relic set bonuses (affects only the wearer).
///
/// Call this AFTER applying relic main stats so SPD flat bonuses from Feet are
/// already in `base_stats` before computing speed-based conditionals.
/// Planar ornament set bonuses are handled separately in `planars::apply_set_bonuses`.
pub fn apply_set_bonuses(member: &mut TeamMember, relics: &[IncomingRelic]) {
    macro_rules! apply_set {
        ($id:literal, $module:ident) => {{
            let n = count_set(relics, $id);
            if n > 0 { $module::apply(member, n); }
        }};
    }

    apply_set!("band_of_sizzling_thunder",              band_of_sizzling_thunder);
    apply_set!("champion_of_streetwise_boxing",         champion_of_streetwise_boxing);
    apply_set!("diviner_of_distant_reach",              diviner_of_distant_reach);
    apply_set!("eagle_of_twilight_line",                eagle_of_twilight_line);
    apply_set!("ever_glorious_magical_girl",            ever_glorious_magical_girl);
    apply_set!("firesmith_of_lava_forging",             firesmith_of_lava_forging);
    apply_set!("genius_of_brilliant_stars",             genius_of_brilliant_stars);
    apply_set!("guard_of_wuthering_snow",               guard_of_wuthering_snow);
    apply_set!("hero_of_triumphant_song",               hero_of_triumphant_song);
    apply_set!("hunter_of_glacial_forest",              hunter_of_glacial_forest);
    apply_set!("iron_cavalry_against_the_scourge",      iron_cavalry_against_the_scourge);
    apply_set!("knight_of_purity_palace",               knight_of_purity_palace);
    apply_set!("longevous_disciple",                    longevous_disciple);
    apply_set!("messenger_traversing_hackerspace",      messenger_traversing_hackerspace);
    apply_set!("musketeer_of_wild_wheat",               musketeer);
    apply_set!("passerby_of_wandering_cloud",           passerby_of_wandering_cloud);
    apply_set!("pioneer_diver_of_dead_waters",          pioneer);
    apply_set!("poet_of_mourning_collapse",             poet_of_mourning_collapse);
    apply_set!("prisoner_in_deep_confinement",          prisoner_in_deep_confinement);
    apply_set!("sacerdos_relived_ordeal",               sacerdos_relived_ordeal);
    apply_set!("scholar_lost_in_erudition",             scholar_lost_in_erudition);
    apply_set!("self_enshrouded_recluse",               self_enshrouded_recluse);
    apply_set!("the_ashblazing_grand_duke",             the_ashblazing_grand_duke);
    apply_set!("the_wind_soaring_valorous",             the_wind_soaring_valorous);
    apply_set!("thief_of_shooting_meteor",              thief_of_shooting_meteor);
    apply_set!("warrior_goddess_of_sun_and_thunder",    warrior_goddess_of_sun_and_thunder);
    apply_set!("wastelander_of_banditry_desert",        wastelander_of_banditry_desert);
    apply_set!("watchmaker_master_of_dream_machinations", watchmaker_master_of_dream_machinations);
    apply_set!("wavestrider_captain",                   wavestrider_captain);
    apply_set!("world_remaking_deliverer",              world_remaking_deliverer);
}

/// Apply team-wide relic set bonuses (affects all allies).
///
/// Must be called after per-character bonuses are applied for all members,
/// so that the wearer detection (≥4 pieces) is accurate.
pub fn apply_team_set_bonuses(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    messenger_traversing_hackerspace::apply_team(team, relic_lists);
    sacerdos_relived_ordeal::apply_team(team, relic_lists);
    self_enshrouded_recluse::apply_team(team, relic_lists);
    warrior_goddess_of_sun_and_thunder::apply_team(team, relic_lists);
    watchmaker_master_of_dream_machinations::apply_team(team, relic_lists);
    world_remaking_deliverer::apply_team(team, relic_lists);
}

// ─── Simulation-time relic hooks ─────────────────────────────────────────────

/// Fire relic effects that trigger at the start of each ally turn.
/// Called from `execute_ally_turn` before the character acts.
pub fn apply_turn_start_effects(member: &mut TeamMember) {
    // Guard of Wuthering Snow 4p: if HP ≤ 50%, restore 8% Max HP + 5 Energy.
    let guard_4p = member.relics.iter().filter(|r| r.set_id == "guard_of_wuthering_snow").count() >= 4;
    if guard_4p && member.max_hp > 0.0 && member.hp / member.max_hp <= 0.5 {
        let heal = member.max_hp * 0.08;
        member.hp = (member.hp + heal).min(member.max_hp);
        let err_mult = 1.0 + member.buffs.energy_regen_rate / 100.0;
        member.energy = (member.energy + 5.0 * err_mult).min(member.max_energy);
    }

    // Hunter of Glacial Forest 4p: decrement post-ult CRIT DMG window.
    if let Some(w) = member.stacks.get_mut("hunter_window") {
        if *w > 0.0 { *w -= 1.0; }
    }

    // Band of Sizzling Thunder 4p: decrement post-Skill ATK window.
    if let Some(w) = member.stacks.get_mut("band_skill_window") {
        if *w > 0.0 { *w -= 1.0; }
    }

    // Sacerdos' Relived Ordeal 4p: decrement CRIT DMG buff window; clear stacks when expired.
    if let Some(w) = member.stacks.get_mut("sacerdos_cdmg_window") {
        if *w > 0.0 { *w -= 1.0; }
        if *w <= 0.0 {
            member.stacks.remove("sacerdos_cdmg_bonus");
        }
    }

    // Watchmaker 4p: decrement Break Effect window; clear buff when expired.
    if let Some(w) = member.stacks.get_mut("watchmaker_be_window") {
        if *w > 0.0 { *w -= 1.0; }
        if *w <= 0.0 {
            member.stacks.remove("watchmaker_be_window");
        }
    }

    // Messenger Traversing Hackerspace 4p: decrement SPD window.
    // The +12% SPD is stored persistently in buffs.speed_percent; remove when expired.
    if let Some(w) = member.stacks.get("messenger_spd_window").copied() {
        if w > 0.0 {
            member.stacks.insert("messenger_spd_window".to_string(), w - 1.0);
            if w - 1.0 <= 0.0 {
                member.buffs.speed_percent -= 12.0;
                member.stacks.remove("messenger_spd_window");
            }
        }
    }

    // Longevous Disciple 4p: decrement CRIT Rate stack window; clear stacks when expired.
    if let Some(w) = member.stacks.get("longevous_window").copied() {
        if w > 0.0 {
            member.stacks.insert("longevous_window".to_string(), w - 1.0);
            if w - 1.0 <= 0.0 {
                member.stacks.remove("longevous_stacks");
                member.stacks.remove("longevous_window");
            }
        }
    }

    // The Ashblazing Grand Duke 4p: decrement FUA stack window; clear stacks when expired.
    if let Some(w) = member.stacks.get("ashblazing_window").copied() {
        if w > 0.0 {
            member.stacks.insert("ashblazing_window".to_string(), w - 1.0);
            if w - 1.0 <= 0.0 {
                member.stacks.remove("ashblazing_stacks");
                member.stacks.remove("ashblazing_window");
            }
        }
    }

    // The Wind-Soaring Valorous 4p: decrement post-FUA Ult DMG window.
    if let Some(w) = member.stacks.get("wind_soaring_fua_window").copied() {
        if w > 0.0 {
            member.stacks.insert("wind_soaring_fua_window".to_string(), w - 1.0);
            if w - 1.0 <= 0.0 {
                member.stacks.remove("wind_soaring_fua_window");
            }
        }
    }
}

/// Apply conditional relic bonuses that depend on live combat state.
///
/// Called just before each action, after the buffs snapshot is taken.
/// Any mutations to `member.buffs` are temporary — the snapshot restores them.
/// Mutations to `member.stacks` are permanent (stacks persist across turns).
///
/// `is_ult` must be `true` when this is called for an Ultimate action.
pub fn apply_action_conditional_buffs(member: &mut TeamMember, target: Option<&SimEnemy>, action_type: &ActionType) {
    let is_ult   = *action_type == ActionType::Ultimate;
    let is_skill = *action_type == ActionType::Skill;

    // Clone relic list to avoid simultaneous borrow of member.
    let relics: Vec<_> = member.relics.clone();

    // ── Pioneer Diver of Dead Waters ──────────────────────────────────────────
    // 2p: DMG +12% when hitting debuffed enemies.
    // 4p: CRIT Rate +4%, CRIT DMG +24% when target has ≥2 debuffs.
    let pioneer_count = count_set(&relics, "pioneer_diver_of_dead_waters");
    if pioneer_count >= 2 {
        if let Some(t) = target {
            if !t.active_debuffs.is_empty() {
                member.buffs.dmg_boost += 12.0;
            }
            if pioneer_count >= 4 && t.active_debuffs.len() >= 2 {
                member.buffs.crit_rate += 4.0;
                member.buffs.crit_dmg  += 24.0;
            }
        }
    }

    // ── Wastelander of Banditry Desert ────────────────────────────────────────
    // 4p: CRIT Rate +10% vs debuffed enemies; CRIT DMG +20% vs Imprisoned enemies.
    if count_set(&relics, "wastelander_of_banditry_desert") >= 4 {
        if let Some(t) = target {
            if !t.active_debuffs.is_empty() {
                member.buffs.crit_rate += 10.0;
            }
            if t.active_debuffs.contains_key("Imprisoned") {
                member.buffs.crit_dmg += 20.0;
            }
        }
    }

    // ── Genius of Brilliant Stars ─────────────────────────────────────────────
    // 4p extra: +10% DEF ignore when target has Quantum Weakness (on top of the
    // guaranteed 10% already applied at setup).
    if count_set(&relics, "genius_of_brilliant_stars") >= 4 && member.element == "Quantum" {
        if let Some(t) = target {
            if t.weaknesses.contains(&"Quantum".to_string()) {
                member.buffs.def_ignore += 10.0;
            }
        }
    }

    // ── Hunter of Glacial Forest ──────────────────────────────────────────────
    // 4p: CRIT DMG +25% while post-ult window is active (decremented each turn start).
    if count_set(&relics, "hunter_of_glacial_forest") >= 4 {
        let window = member.stacks.get("hunter_window").copied().unwrap_or(0.0);
        if window > 0.0 {
            member.buffs.crit_dmg += 25.0;
        }
    }

    // ── Firesmith of Lava-Forging ─────────────────────────────────────────────
    // 4p: Fire DMG +12% for the one attack immediately after using Ult.
    // Consuming the window (stacks) is permanent; the DMG bonus is snapshotted.
    if count_set(&relics, "firesmith_of_lava_forging") >= 4 && member.element == "Fire" {
        let window = member.stacks.get("firesmith_ult_window").copied().unwrap_or(0.0);
        if window > 0.0 {
            member.buffs.dmg_boost += 12.0;
            member.stacks.remove("firesmith_ult_window"); // one-shot — consume now
        }
    }

    // ── Champion of Streetwise Boxing ─────────────────────────────────────────
    // 4p: ATK +5% per combat stack (max 5 stacks, gained on attack or being hit).
    if count_set(&relics, "champion_of_streetwise_boxing") >= 4 {
        let stacks = member.stacks.get("champion_stacks").copied().unwrap_or(0.0).min(5.0);
        if stacks > 0.0 {
            member.buffs.atk_percent += stacks * 5.0;
        }
    }

    // ── Wavestrider Captain ───────────────────────────────────────────────────
    // 4p: ATK +48% on the Ult turn when consuming 2 "Help" stacks.
    if is_ult && count_set(&relics, "wavestrider_captain") >= 4 {
        let stacks = member.stacks.get("wavestrider_stacks").copied().unwrap_or(0.0);
        if stacks >= 2.0 {
            member.buffs.atk_percent += 48.0;
            member.stacks.remove("wavestrider_stacks"); // consume permanently
        }
    }

    // ── Sigonia, the Unclaimed Desolation ─────────────────────────────────────
    // 2p kill stacking: CRIT DMG +4% per kill stack (max 10).
    if count_set(&relics, "sigonia_the_unclaimed_desolation") >= 2 {
        let stacks = member.stacks.get("sigonia_stacks").copied().unwrap_or(0.0).min(10.0);
        if stacks > 0.0 {
            member.buffs.crit_dmg += stacks * 4.0;
        }
    }

    // ── Band of Sizzling Thunder ──────────────────────────────────────────────
    // 4p: ATK +20% for 1 turn after using Skill.
    if count_set(&relics, "band_of_sizzling_thunder") >= 4 {
        let window = member.stacks.get("band_skill_window").copied().unwrap_or(0.0);
        if window > 0.0 {
            member.buffs.atk_percent += 20.0;
        }
    }

    // ── Sacerdos' Relived Ordeal ──────────────────────────────────────────────
    // 4p: CRIT DMG bonus granted by a Sacerdos wearer's Skill/Ult (set via on_action_used).
    let sacerdos_bonus = member.stacks.get("sacerdos_cdmg_bonus").copied().unwrap_or(0.0);
    if sacerdos_bonus > 0.0 {
        member.buffs.crit_dmg += sacerdos_bonus;
    }

    // ── Watchmaker, Master of Dream Machinations ─────────────────────────────
    // 4p: Break Effect +30% team buff (set via on_action_used for ult).
    if member.stacks.get("watchmaker_be_window").copied().unwrap_or(0.0) > 0.0 {
        member.buffs.break_effect += 30.0;
    }

    // ── Prisoner in Deep Confinement ──────────────────────────────────────────
    // 4p: +6% DEF ignore per DoT on target (Burn, Bleed, Shock, Wind Shear,
    //     Arcana, Entanglement) — max 3 DoTs = +18%.
    if count_set(&relics, "prisoner_in_deep_confinement") >= 4 {
        if let Some(t) = target {
            let dot_count = t.active_debuffs.values()
                .filter(|e| {
                    matches!(
                        e.stat.as_deref().unwrap_or(""),
                        "Burn" | "Bleed" | "Shock" | "Wind Shear" | "Arcana" | "Entanglement"
                    )
                })
                .count()
                .min(3) as f64;
            if dot_count > 0.0 {
                member.buffs.def_ignore += dot_count * 6.0;
            }
        }
    }

    // ── Longevous Disciple ────────────────────────────────────────────────────
    // 4p: +8% CRIT Rate per active stack (up to 2 stacks via on_hit_taken).
    if count_set(&relics, "longevous_disciple") >= 4 {
        let stacks = member.stacks.get("longevous_stacks").copied().unwrap_or(0.0).min(2.0);
        if stacks > 0.0 {
            member.buffs.crit_rate += stacks * 8.0;
        }
    }

    // ── The Ashblazing Grand Duke ─────────────────────────────────────────────
    // 4p: ATK +6% per FUA stack (0-8 stacks; stacks reset on each new FUA via on_follow_up_start).
    if count_set(&relics, "the_ashblazing_grand_duke") >= 4 {
        let stacks = member.stacks.get("ashblazing_stacks").copied().unwrap_or(0.0).min(8.0);
        if stacks > 0.0 {
            member.buffs.atk_percent += stacks * 6.0;
        }
    }

    // ── The Wind-Soaring Valorous ─────────────────────────────────────────────
    // 4p: Ult DMG +36% for 1 turn after a follow-up attack.
    if is_ult && count_set(&relics, "the_wind_soaring_valorous") >= 4 {
        if member.stacks.get("wind_soaring_fua_window").copied().unwrap_or(0.0) > 0.0 {
            member.buffs.ult_dmg_boost += 36.0;
        }
    }

    // ── Scholar Lost in Erudition ─────────────────────────────────────────────
    // 4p: Skill DMG +25% for the next Skill after using Ultimate (one-shot window).
    if is_skill && count_set(&relics, "scholar_lost_in_erudition") >= 4 {
        if member.stacks.get("scholar_ult_window").copied().unwrap_or(0.0) > 0.0 {
            member.buffs.skill_dmg_boost += 25.0;
            member.stacks.remove("scholar_ult_window");
        }
    }

    // ── Self-Enshrouded Recluse ───────────────────────────────────────────────
    // 4p: CRIT DMG +15% when the wearer's ally has an active shield from the Recluse wearer.
    // Approximation: if a Recluse 4p wearer is in the team (flagged at setup) and this
    // member currently has any shield, grant the bonus.
    if member.stacks.get("recluse_crit_available").copied().unwrap_or(0.0) >= 1.0
        && member.shield > 0.0
    {
        member.buffs.crit_dmg += 15.0;
    }
}

/// Called after each action fires (after buffs snapshot is restored).
/// Sets per-turn windows and team-wide buffs for relic effects that activate on specific actions.
///
/// `action_type` — the type of action just completed.
/// Team-wide buffs (Messenger SPD, Watchmaker BE, Sacerdos CD) are applied to all members.
pub fn on_action_used(team: &mut Vec<TeamMember>, wearer_idx: usize, action_type: &ActionType) {
    let relics: Vec<_> = team[wearer_idx].relics.clone();
    let is_ult   = *action_type == ActionType::Ultimate;
    let is_skill = *action_type == ActionType::Skill;

    // ── Per-wearer effects ────────────────────────────────────────────────────

    // Band of Sizzling Thunder 4p: ATK +20% for 1 turn after Skill.
    if is_skill && count_set(&relics, "band_of_sizzling_thunder") >= 4 {
        team[wearer_idx].stacks.insert("band_skill_window".to_string(), 1.0);
    }

    // Hunter of Glacial Forest 4p: CRIT DMG +25% for 2 turns after Ult.
    if is_ult && count_set(&relics, "hunter_of_glacial_forest") >= 4 {
        team[wearer_idx].stacks.insert("hunter_window".to_string(), 2.0);
    }

    // Scholar Lost in Erudition 4p: Skill DMG +25% for the next Skill after Ult.
    if is_ult && count_set(&relics, "scholar_lost_in_erudition") >= 4 {
        team[wearer_idx].stacks.insert("scholar_ult_window".to_string(), 1.0);
    }

    // Firesmith of Lava-Forging 4p: Fire DMG +12% for next attack after Ult.
    if is_ult && count_set(&relics, "firesmith_of_lava_forging") >= 4
        && team[wearer_idx].element == "Fire"
    {
        team[wearer_idx].stacks.insert("firesmith_ult_window".to_string(), 1.0);
    }

    // ── Team-wide effects ─────────────────────────────────────────────────────

    // Sacerdos' Relived Ordeal 4p: CRIT DMG +18% to all non-wearer allies (max 2 stacks)
    // for 2 turns, triggered by Skill or Ult.
    if (is_skill || is_ult) && count_set(&relics, "sacerdos_relived_ordeal") >= 4 {
        let n = team.len();
        for i in 0..n {
            if i == wearer_idx { continue; }
            // Accumulate up to 36% total CRIT DMG (2 stacks × 18%).
            let current = team[i].stacks.get("sacerdos_cdmg_bonus").copied().unwrap_or(0.0);
            let new_bonus = (current + 18.0).min(36.0);
            team[i].stacks.insert("sacerdos_cdmg_bonus".to_string(), new_bonus);
            // Refresh or start the 2-turn window.
            team[i].stacks.insert("sacerdos_cdmg_window".to_string(), 2.0);
        }
    }

    if is_ult {
        // Messenger Traversing Hackerspace 4p: all allies SPD +12% for 1 turn.
        // Applied as a persistent buff to buffs.speed_percent (affects AV re-scheduling).
        // If the buff is already active, refresh the window without double-adding.
        if count_set(&relics, "messenger_traversing_hackerspace") >= 4 {
            for member in team.iter_mut() {
                let active = member.stacks.get("messenger_spd_window").copied().unwrap_or(0.0) > 0.0;
                if !active {
                    member.buffs.speed_percent += 12.0;
                }
                // Window = 2 so turn-start decrement fires once before the next action.
                member.stacks.insert("messenger_spd_window".to_string(), 2.0);
            }
        }

        // Watchmaker, Master of Dream Machinations 4p: all allies Break Effect +30% for 2 turns.
        if count_set(&relics, "watchmaker_master_of_dream_machinations") >= 4 {
            for member in team.iter_mut() {
                member.stacks.insert("watchmaker_be_window".to_string(), 2.0);
            }
        }
    }
}

/// Called when an ally successfully lands an attack.
/// Increments combat-state stacks tied to attacking.
pub fn on_attack_hit(member: &mut TeamMember) {
    let relics: Vec<_> = member.relics.clone();

    // Champion of Streetwise Boxing 4p: +1 stack after attacking (max 5).
    if count_set(&relics, "champion_of_streetwise_boxing") >= 4 {
        let s = member.stacks.entry("champion_stacks".to_string()).or_insert(0.0);
        *s = (*s + 1.0).min(5.0);
    }
}

/// Called when an ally takes damage.
/// Increments combat-state stacks triggered by being hit.
pub fn on_hit_taken(member: &mut TeamMember) {
    let relics: Vec<_> = member.relics.clone();

    // Champion of Streetwise Boxing 4p: +1 stack when attacked (max 5).
    if count_set(&relics, "champion_of_streetwise_boxing") >= 4 {
        let s = member.stacks.entry("champion_stacks".to_string()).or_insert(0.0);
        *s = (*s + 1.0).min(5.0);
    }

    // Wavestrider Captain 4p: +1 "Help" stack when targeted by an ability (max 2).
    // Note: also triggered via on_ally_targeted for ally Skill/Ult targeting this member.
    if count_set(&relics, "wavestrider_captain") >= 4 {
        let s = member.stacks.entry("wavestrider_stacks".to_string()).or_insert(0.0);
        *s = (*s + 1.0).min(2.0);
    }

    // Longevous Disciple 4p: +1 CRIT Rate stack (max 2) when hit; refresh 2-turn window.
    if count_set(&relics, "longevous_disciple") >= 4 {
        let s = member.stacks.entry("longevous_stacks".to_string()).or_insert(0.0);
        *s = (*s + 1.0).min(2.0);
        member.stacks.insert("longevous_window".to_string(), 2.0);
    }
}

/// Called when any ally's ability targets this specific ally member (heals, buffs, etc.).
/// Wavestrider gains stacks from being targeted by allies, not just from being hit.
pub fn on_ally_targeted(member: &mut TeamMember) {
    let relics: Vec<_> = member.relics.clone();

    // Wavestrider Captain 4p: +1 "Help" stack when targeted by ally ability (max 2).
    if count_set(&relics, "wavestrider_captain") >= 4 {
        let s = member.stacks.entry("wavestrider_stacks".to_string()).or_insert(0.0);
        *s = (*s + 1.0).min(2.0);
    }
}

/// Called when any enemy is killed.
/// Updates kill-triggered stacks for all members.
pub fn on_enemy_killed(team: &mut [TeamMember]) {
    for member in team.iter_mut() {
        let relics: Vec<_> = member.relics.clone();

        // Sigonia, the Unclaimed Desolation: +1 CRIT DMG stack per kill (max 10).
        if count_set(&relics, "sigonia_the_unclaimed_desolation") >= 2 {
            let s = member.stacks.entry("sigonia_stacks".to_string()).or_insert(0.0);
            *s = (*s + 1.0).min(10.0);
        }
    }
}

// ─── Follow-up attack hooks ───────────────────────────────────────────────────

/// Called at the start of a follow-up attack sequence, before the first hit.
/// Resets Ashblazing Grand Duke stacks so they build up fresh per hit.
pub fn on_follow_up_start(team: &mut Vec<TeamMember>, wearer_idx: usize) {
    let relics: Vec<_> = team[wearer_idx].relics.clone();

    // The Ashblazing Grand Duke 4p: stacks reset on each new follow-up.
    if count_set(&relics, "the_ashblazing_grand_duke") >= 4 {
        team[wearer_idx].stacks.insert("ashblazing_stacks".to_string(), 0.0);
    }
}

/// Called for each individual hit within a follow-up attack.
/// Increments Ashblazing Grand Duke ATK stacks (+1 per hit, max 8, window 3 turns).
pub fn on_follow_up_hit(team: &mut Vec<TeamMember>, wearer_idx: usize) {
    let relics: Vec<_> = team[wearer_idx].relics.clone();

    if count_set(&relics, "the_ashblazing_grand_duke") >= 4 {
        let s = team[wearer_idx].stacks.entry("ashblazing_stacks".to_string()).or_insert(0.0);
        *s = (*s + 1.0).min(8.0);
        team[wearer_idx].stacks.insert("ashblazing_window".to_string(), 3.0);
    }
}

/// Called after a follow-up attack sequence completes.
/// Sets Wind-Soaring Valorous post-FUA Ult DMG window (1 turn).
pub fn on_follow_up_end(team: &mut Vec<TeamMember>, wearer_idx: usize) {
    let relics: Vec<_> = team[wearer_idx].relics.clone();

    // The Wind-Soaring Valorous 4p: Ult DMG +36% for 1 turn after follow-up.
    if count_set(&relics, "the_wind_soaring_valorous") >= 4 {
        team[wearer_idx].stacks.insert("wind_soaring_fua_window".to_string(), 1.0);
    }
}

/// Fire relic effects that trigger at battle start (before first action).
/// Called from `run_simulation` after all set bonuses are applied.
/// Returns the number of bonus Skill Points to add to the team pool.
pub fn apply_battle_start_effects(team: &[TeamMember]) -> i32 {
    let mut bonus_sp = 0i32;
    for member in team {
        // Passerby of Wandering Cloud 4p: +1 Skill Point at battle start.
        let passerby_4p = member.relics.iter()
            .filter(|r| r.set_id == "passerby_of_wandering_cloud").count() >= 4;
        if passerby_4p {
            bonus_sp += 1;
        }
    }
    bonus_sp
}

// ─── Relic optimizer types ────────────────────────────────────────────────────

/// Discrete relic configuration: set choices + main stat choices per variable slot.
/// Head (flat_hp) and Hands (flat_atk) have fixed main stats, so they are omitted.
#[derive(Debug, Clone)]
pub struct RelicConfig {
    /// e.g. "musketeer_4p" | "pioneer_2p" | "none"
    pub relic_set:    String,
    /// e.g. "fleet_2p" | "space_sealing_2p" | "none"
    pub ornament_set: String,
    pub body_main:    String,
    pub feet_main:    String,
    pub sphere_main:  String,
    pub rope_main:    String,
}

/// All relic set options used by the optimizer search space.
/// Each entry is ("display_label", "internal_set_id", piece_count).
const RELIC_SETS: &[(&str, &str, usize)] = &[
    // 4-piece sets
    ("Band 4p",              "band_of_sizzling_thunder",              4),
    ("Champion 4p",          "champion_of_streetwise_boxing",         4),
    ("Diviner 4p",           "diviner_of_distant_reach",              4),
    ("Eagle 4p",             "eagle_of_twilight_line",                4),
    ("Magical Girl 4p",      "ever_glorious_magical_girl",            4),
    ("Firesmith 4p",         "firesmith_of_lava_forging",             4),
    ("Genius 4p",            "genius_of_brilliant_stars",             4),
    ("Guard 4p",             "guard_of_wuthering_snow",               4),
    ("Hero 4p",              "hero_of_triumphant_song",               4),
    ("Hunter 4p",            "hunter_of_glacial_forest",              4),
    ("Iron Cavalry 4p",      "iron_cavalry_against_the_scourge",      4),
    ("Knight 4p",            "knight_of_purity_palace",               4),
    ("Longevous 4p",         "longevous_disciple",                    4),
    ("Messenger 4p",         "messenger_traversing_hackerspace",      4),
    ("Musketeer 4p",         "musketeer_of_wild_wheat",               4),
    ("Passerby 4p",          "passerby_of_wandering_cloud",           4),
    ("Pioneer 4p",           "pioneer_diver_of_dead_waters",          4),
    ("Poet 4p",              "poet_of_mourning_collapse",             4),
    ("Prisoner 4p",          "prisoner_in_deep_confinement",          4),
    ("Sacerdos 4p",          "sacerdos_relived_ordeal",               4),
    ("Scholar 4p",           "scholar_lost_in_erudition",             4),
    ("Recluse 4p",           "self_enshrouded_recluse",               4),
    ("Ashblazing 4p",        "the_ashblazing_grand_duke",             4),
    ("Wind-Soaring 4p",      "the_wind_soaring_valorous",             4),
    ("Thief 4p",             "thief_of_shooting_meteor",              4),
    ("Warrior Goddess 4p",   "warrior_goddess_of_sun_and_thunder",    4),
    ("Wastelander 4p",       "wastelander_of_banditry_desert",        4),
    ("Watchmaker 4p",        "watchmaker_master_of_dream_machinations", 4),
    ("Wavestrider 4p",       "wavestrider_captain",                   4),
    ("Deliverer 4p",         "world_remaking_deliverer",              4),
    // 2-piece sets
    ("Band 2p",              "band_of_sizzling_thunder",              2),
    ("Champion 2p",          "champion_of_streetwise_boxing",         2),
    ("Diviner 2p",           "diviner_of_distant_reach",              2),
    ("Eagle 2p",             "eagle_of_twilight_line",                2),
    ("Magical Girl 2p",      "ever_glorious_magical_girl",            2),
    ("Firesmith 2p",         "firesmith_of_lava_forging",             2),
    ("Genius 2p",            "genius_of_brilliant_stars",             2),
    ("Guard 2p",             "guard_of_wuthering_snow",               2),
    ("Hero 2p",              "hero_of_triumphant_song",               2),
    ("Hunter 2p",            "hunter_of_glacial_forest",              2),
    ("Iron Cavalry 2p",      "iron_cavalry_against_the_scourge",      2),
    ("Knight 2p",            "knight_of_purity_palace",               2),
    ("Longevous 2p",         "longevous_disciple",                    2),
    ("Messenger 2p",         "messenger_traversing_hackerspace",      2),
    ("Musketeer 2p",         "musketeer_of_wild_wheat",               2),
    ("Passerby 2p",          "passerby_of_wandering_cloud",           2),
    ("Pioneer 2p",           "pioneer_diver_of_dead_waters",          2),
    ("Poet 2p",              "poet_of_mourning_collapse",             2),
    ("Prisoner 2p",          "prisoner_in_deep_confinement",          2),
    ("Sacerdos 2p",          "sacerdos_relived_ordeal",               2),
    ("Scholar 2p",           "scholar_lost_in_erudition",             2),
    ("Recluse 2p",           "self_enshrouded_recluse",               2),
    ("Ashblazing 2p",        "the_ashblazing_grand_duke",             2),
    ("Wind-Soaring 2p",      "the_wind_soaring_valorous",             2),
    ("Thief 2p",             "thief_of_shooting_meteor",              2),
    ("Warrior Goddess 2p",   "warrior_goddess_of_sun_and_thunder",    2),
    ("Wastelander 2p",       "wastelander_of_banditry_desert",        2),
    ("Watchmaker 2p",        "watchmaker_master_of_dream_machinations", 2),
    ("Wavestrider 2p",       "wavestrider_captain",                   2),
    ("Deliverer 2p",         "world_remaking_deliverer",              2),
    // No set
    ("No Relic Set",         "none",                                  0),
];

/// Generate all discrete relic configurations for the optimizer.
///
/// N_relic_sets × N_ornament_sets × 7 body × 4 feet × 10 sphere × 5 rope
pub fn all_relic_configs() -> Vec<RelicConfig> {
    const ORNAMENT_SETS: &[&str] = &[
        "amphoreus_2p",
        "arcadia_2p",
        "belobog_2p",
        "bone_collection_2p",
        "broken_keel_2p",
        "celestial_differentiator_2p",
        "city_of_converging_stars_2p",
        "duran_2p",
        "glamoth_2p",
        "fleet_2p",
        "forge_kalpagni_2p",
        "giant_tree_2p",
        "inert_salsotto_2p",
        "izumo_2p",
        "lushaka_2p",
        "pan_cosmic_2p",
        "penacony_2p",
        "punklorde_2p",
        "revelry_2p",
        "rutilant_arena_2p",
        "sigonia_2p",
        "space_sealing_2p",
        "sprightly_vonwacq_2p",
        "talia_2p",
        "tengoku_2p",
        "banamusement_2p",
        "none",
    ];
    const BODY_MAINS:    &[&str] = &[
        "hp_percent", "atk_percent", "def_percent",
        "crit_rate", "crit_dmg", "outgoing_healing", "effect_hit_rate",
    ];
    const FEET_MAINS:    &[&str] = &["hp_percent", "atk_percent", "def_percent", "speed"];
    const SPHERE_MAINS:  &[&str] = &[
        "hp_percent", "atk_percent", "def_percent",
        "physical_dmg", "fire_dmg", "ice_dmg", "lightning_dmg",
        "wind_dmg", "quantum_dmg", "imaginary_dmg",
    ];
    const ROPE_MAINS:    &[&str] = &["hp_percent", "atk_percent", "def_percent", "break_effect", "err"];

    let capacity = RELIC_SETS.len() * ORNAMENT_SETS.len()
        * BODY_MAINS.len() * FEET_MAINS.len()
        * SPHERE_MAINS.len() * ROPE_MAINS.len();
    let mut configs = Vec::with_capacity(capacity);

    for &(label, _set_id, _count) in RELIC_SETS {
        for &os in ORNAMENT_SETS {
            for &bm in BODY_MAINS {
                for &fm in FEET_MAINS {
                    for &sm in SPHERE_MAINS {
                        for &rm in ROPE_MAINS {
                            configs.push(RelicConfig {
                                relic_set:    label.to_string(),
                                ornament_set: os.to_string(),
                                body_main:    bm.to_string(),
                                feet_main:    fm.to_string(),
                                sphere_main:  sm.to_string(),
                                rope_main:    rm.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    configs
}

/// Convert a `RelicConfig` into 6 `IncomingRelic` pieces ready to equip.
///
/// Head and Hands always have fixed main stats (flat_hp / flat_atk).
/// For 2-piece configs, only Head+Hands carry the set; Body+Feet use "generic".
pub fn config_to_relics(config: &RelicConfig) -> Vec<IncomingRelic> {
    // Find the (set_id, piece_count) for this label.
    let (set_id, piece_count) = RELIC_SETS.iter()
        .find(|(label, _, _)| *label == config.relic_set)
        .map(|(_, id, count)| (*id, *count))
        .unwrap_or(("none", 0));

    let (r_head, r_hands, r_body, r_feet) = if piece_count >= 4 {
        (set_id, set_id, set_id, set_id)
    } else if piece_count >= 2 {
        (set_id, set_id, "generic", "generic")
    } else {
        ("generic", "generic", "generic", "generic")
    };

    let (r_sphere, r_rope) = match config.ornament_set.as_str() {
        "amphoreus_2p"              => ("amphoreus_the_eternal_land",           "amphoreus_the_eternal_land"),
        "arcadia_2p"                => ("arcadia_of_woven_dreams",              "arcadia_of_woven_dreams"),
        "belobog_2p"                => ("belobog_of_the_architects",            "belobog_of_the_architects"),
        "bone_collection_2p"        => ("bone_collections_serene_demesne",      "bone_collections_serene_demesne"),
        "broken_keel_2p"            => ("broken_keel",                          "broken_keel"),
        "celestial_differentiator_2p" => ("celestial_differentiator",           "celestial_differentiator"),
        "city_of_converging_stars_2p" => ("city_of_converging_stars",           "city_of_converging_stars"),
        "duran_2p"                  => ("duran_dynasty_of_running_wolves",       "duran_dynasty_of_running_wolves"),
        "glamoth_2p"                => ("firmament_frontline_glamoth",           "firmament_frontline_glamoth"),
        "fleet_2p"                  => ("fleet_of_the_ageless",                 "fleet_of_the_ageless"),
        "forge_kalpagni_2p"         => ("forge_of_the_kalpagni_lantern",        "forge_of_the_kalpagni_lantern"),
        "giant_tree_2p"             => ("giant_tree_of_rapt_brooding",          "giant_tree_of_rapt_brooding"),
        "inert_salsotto_2p"         => ("inert_salsotto",                       "inert_salsotto"),
        "izumo_2p"                  => ("izumo_gensei_and_takama_divine_realm",  "izumo_gensei_and_takama_divine_realm"),
        "lushaka_2p"                => ("lushaka_the_sunken_seas",              "lushaka_the_sunken_seas"),
        "pan_cosmic_2p"             => ("pan_cosmic_commercial_enterprise",     "pan_cosmic_commercial_enterprise"),
        "penacony_2p"               => ("penacony_land_of_the_dreams",          "penacony_land_of_the_dreams"),
        "punklorde_2p"              => ("punklorde_stage_zero",                 "punklorde_stage_zero"),
        "revelry_2p"                => ("revelry_by_the_sea",                   "revelry_by_the_sea"),
        "rutilant_arena_2p"         => ("rutilant_arena",                       "rutilant_arena"),
        "sigonia_2p"                => ("sigonia_the_unclaimed_desolation",      "sigonia_the_unclaimed_desolation"),
        "space_sealing_2p"          => ("space_sealing_station",                "space_sealing_station"),
        "sprightly_vonwacq_2p"      => ("sprightly_vonwacq",                    "sprightly_vonwacq"),
        "talia_2p"                  => ("talia_kingdom_of_banditry",            "talia_kingdom_of_banditry"),
        "tengoku_2p"                => ("tengoku_livestream",                   "tengoku_livestream"),
        "banamusement_2p"           => ("the_wondrous_banamusement_park",       "the_wondrous_banamusement_park"),
        _                           => ("generic",                              "generic"),
    };

    vec![
        IncomingRelic { set_id: r_head.to_string(),   slot: RelicSlot::Head,         main_stat: "flat_hp".to_string()      },
        IncomingRelic { set_id: r_hands.to_string(),  slot: RelicSlot::Hands,        main_stat: "flat_atk".to_string()     },
        IncomingRelic { set_id: r_body.to_string(),   slot: RelicSlot::Body,         main_stat: config.body_main.clone()   },
        IncomingRelic { set_id: r_feet.to_string(),   slot: RelicSlot::Feet,         main_stat: config.feet_main.clone()   },
        IncomingRelic { set_id: r_sphere.to_string(), slot: RelicSlot::PlanarSphere, main_stat: config.sphere_main.clone() },
        IncomingRelic { set_id: r_rope.to_string(),   slot: RelicSlot::LinkRope,     main_stat: config.rope_main.clone()   },
    ]
}

// ─── Decomposed optimizer types ──────────────────────────────────────────────

/// A (relic set, ornament set) pair, independent of main stat choices.
/// The two-pass optimizer sweeps these 1 647 pairs first, then sweeps
/// the 1 400 main stat combos separately — 755× fewer simulations than the
/// full Cartesian product.
#[derive(Debug, Clone)]
pub struct SetCombo {
    pub relic_set:    String, // display label, e.g. "Pioneer 4p"
    pub ornament_set: String, // short code,   e.g. "broken_keel_2p"
}

/// All 61 × 27 = 1 647 (relic set, ornament set) combinations.
pub fn all_set_combos() -> Vec<SetCombo> {
    const ORNAMENT_CODES: &[&str] = &[
        "amphoreus_2p", "arcadia_2p", "belobog_2p", "bone_collection_2p",
        "broken_keel_2p", "celestial_differentiator_2p", "city_of_converging_stars_2p",
        "duran_2p", "glamoth_2p", "fleet_2p", "forge_kalpagni_2p", "giant_tree_2p",
        "inert_salsotto_2p", "izumo_2p", "lushaka_2p", "pan_cosmic_2p", "penacony_2p",
        "punklorde_2p", "revelry_2p", "rutilant_arena_2p", "sigonia_2p", "space_sealing_2p",
        "sprightly_vonwacq_2p", "talia_2p", "tengoku_2p", "banamusement_2p", "none",
    ];
    let mut out = Vec::with_capacity(RELIC_SETS.len() * ORNAMENT_CODES.len());
    for &(label, _, _) in RELIC_SETS {
        for &os in ORNAMENT_CODES {
            out.push(SetCombo { relic_set: label.to_string(), ornament_set: os.to_string() });
        }
    }
    out
}

/// Body / Feet / Sphere / Rope main stat combination, decoupled from set choices.
#[derive(Debug, Clone)]
pub struct MainStatCombo {
    pub body_main:   String,
    pub feet_main:   String,
    pub sphere_main: String,
    pub rope_main:   String,
}

/// All 7 × 4 × 10 × 5 = 1 400 main stat combinations.
pub fn all_main_stat_combos() -> Vec<MainStatCombo> {
    const BODY:   &[&str] = &[
        "hp_percent", "atk_percent", "def_percent",
        "crit_rate", "crit_dmg", "outgoing_healing", "effect_hit_rate",
    ];
    const FEET:   &[&str] = &["hp_percent", "atk_percent", "def_percent", "speed"];
    const SPHERE: &[&str] = &[
        "hp_percent", "atk_percent", "def_percent",
        "physical_dmg", "fire_dmg", "ice_dmg", "lightning_dmg",
        "wind_dmg", "quantum_dmg", "imaginary_dmg",
    ];
    const ROPE:   &[&str] = &["hp_percent", "atk_percent", "def_percent", "break_effect", "err"];
    let mut out = Vec::with_capacity(7 * 4 * 10 * 5);
    for &b in BODY { for &f in FEET { for &s in SPHERE { for &r in ROPE {
        out.push(MainStatCombo {
            body_main:   b.to_string(),
            feet_main:   f.to_string(),
            sphere_main: s.to_string(),
            rope_main:   r.to_string(),
        });
    }}}}
    out
}

/// Map an ornament-set short code to its display name (used in API responses).
pub fn ornament_display(s: &str) -> &'static str {
    match s {
        "amphoreus_2p"                => "Amphoreus 2p",
        "arcadia_2p"                  => "Arcadia 2p",
        "belobog_2p"                  => "Belobog 2p",
        "bone_collection_2p"          => "Bone Collection 2p",
        "broken_keel_2p"              => "Broken Keel 2p",
        "celestial_differentiator_2p" => "Celestial Differentiator 2p",
        "city_of_converging_stars_2p" => "City of Converging Stars 2p",
        "duran_2p"                    => "Duran 2p",
        "glamoth_2p"                  => "Glamoth 2p",
        "fleet_2p"                    => "Fleet 2p",
        "forge_kalpagni_2p"           => "Forge of Kalpagni 2p",
        "giant_tree_2p"               => "Giant Tree 2p",
        "inert_salsotto_2p"           => "Inert Salsotto 2p",
        "izumo_2p"                    => "Izumo 2p",
        "lushaka_2p"                  => "Lushaka 2p",
        "pan_cosmic_2p"               => "Pan-Cosmic 2p",
        "penacony_2p"                 => "Penacony 2p",
        "punklorde_2p"                => "Punklorde 2p",
        "revelry_2p"                  => "Revelry by the Sea 2p",
        "rutilant_arena_2p"           => "Rutilant Arena 2p",
        "sigonia_2p"                  => "Sigonia 2p",
        "space_sealing_2p"            => "Space Sealing 2p",
        "sprightly_vonwacq_2p"        => "Sprightly Vonwacq 2p",
        "talia_2p"                    => "Talia 2p",
        "tengoku_2p"                  => "Tengoku 2p",
        "banamusement_2p"             => "BananAmusement Park 2p",
        _                             => "No Ornament Set",
    }
}

/// Human-readable summary of a `RelicConfig` for the API response.
pub fn format_relic_config(config: &RelicConfig) -> String {
    let os = match config.ornament_set.as_str() {
        "amphoreus_2p"                => "Amphoreus 2p",
        "arcadia_2p"                  => "Arcadia 2p",
        "belobog_2p"                  => "Belobog 2p",
        "bone_collection_2p"          => "Bone Collection 2p",
        "broken_keel_2p"              => "Broken Keel 2p",
        "celestial_differentiator_2p" => "Celestial Differentiator 2p",
        "city_of_converging_stars_2p" => "City of Converging Stars 2p",
        "duran_2p"                    => "Duran 2p",
        "glamoth_2p"                  => "Glamoth 2p",
        "fleet_2p"                    => "Fleet 2p",
        "forge_kalpagni_2p"           => "Forge of Kalpagni 2p",
        "giant_tree_2p"               => "Giant Tree 2p",
        "inert_salsotto_2p"           => "Inert Salsotto 2p",
        "izumo_2p"                    => "Izumo 2p",
        "lushaka_2p"                  => "Lushaka 2p",
        "pan_cosmic_2p"               => "Pan-Cosmic 2p",
        "penacony_2p"                 => "Penacony 2p",
        "punklorde_2p"                => "Punklorde 2p",
        "revelry_2p"                  => "Revelry by the Sea 2p",
        "rutilant_arena_2p"           => "Rutilant Arena 2p",
        "sigonia_2p"                  => "Sigonia 2p",
        "space_sealing_2p"            => "Space Sealing 2p",
        "sprightly_vonwacq_2p"        => "Sprightly Vonwacq 2p",
        "talia_2p"                    => "Talia 2p",
        "tengoku_2p"                  => "Tengoku 2p",
        "banamusement_2p"             => "BananAmusement Park 2p",
        _                             => "No Ornament Set",
    };
    format!(
        "{} / {} | Body: {} | Feet: {} | Sphere: {} | Rope: {}",
        config.relic_set, os,
        config.body_main, config.feet_main, config.sphere_main, config.rope_main
    )
}
