#![allow(dead_code)]
use crate::ids;
use crate::models::{ActionParams, ActionType, SimEnemy, TeamMember};

/// Level multiplier table (from formulas.ts)
fn level_mult(level: i32) -> f64 {
    match level {
        1  => 54.0000,  2  => 58.0000,  3  => 62.0000,  4  => 67.5264,  5  => 70.5094,
        6  => 73.5228,  7  => 76.5660,  8  => 79.6385,  9  => 82.7395,  10 => 85.8684,
        11 => 91.4944,  12 => 97.0680,  13 => 102.5892, 14 => 108.0579, 15 => 113.4743,
        16 => 118.8383, 17 => 124.1499, 18 => 129.4091, 19 => 134.6159, 20 => 139.7703,
        21 => 149.3323, 22 => 158.8011, 23 => 168.1768, 24 => 177.4594, 25 => 186.6489,
        26 => 195.7452, 27 => 204.7484, 28 => 213.6585, 29 => 222.4754, 30 => 231.1992,
        31 => 246.4276, 32 => 261.1810, 33 => 275.4733, 34 => 289.3179, 35 => 302.7275,
        36 => 315.7144, 37 => 328.2905, 38 => 340.4671, 39 => 352.2554, 40 => 363.6658,
        41 => 408.1240, 42 => 451.7883, 43 => 494.6798, 44 => 536.8188, 45 => 578.2249,
        46 => 618.9172, 47 => 658.9138, 48 => 698.2325, 49 => 736.8905, 50 => 774.9041,
        51 => 871.0599, 52 => 964.8705, 53 => 1056.4206,54 => 1145.7910,55 => 1233.0585,
        56 => 1318.2965,57 => 1401.5750,58 => 1482.9608,59 => 1562.5178,60 => 1640.3068,
        61 => 1752.3215,62 => 1861.9011,63 => 1969.1242,64 => 2074.0659,65 => 2176.7983,
        66 => 2277.3904,67 => 2375.9085,68 => 2472.4160,69 => 2566.9739,70 => 2659.6406,
        71 => 2780.3044,72 => 2898.6022,73 => 3014.6029,74 => 3128.3729,75 => 3239.9758,
        76 => 3349.4730,77 => 3456.9236,78 => 3562.3843,79 => 3665.9099,80 => 3767.5533,
        81 => 3957.8618,82 => 4155.2118,83 => 4359.8638,84 => 4572.0878,85 => 4792.1641,
        86 => 5020.3833,87 => 5257.0466,88 => 5502.4664,89 => 5756.9667,90 => 6020.8836,
        91 => 6294.5654,92 => 6578.3734,93 => 6872.6823,94 => 7177.8806,95 => 7494.3713,
        _  => 3767.5533, // default to level 80
    }
}

/// Break base DMG coefficient by element
fn break_base_coeff(element: &str) -> f64 {
    match element {
        "Physical"  => 2.0,
        "Fire"      => 2.0,
        "Ice"       => 1.0,
        "Lightning" => 1.0,
        "Wind"      => 1.5,
        "Quantum"   => 0.5,
        "Imaginary" => 0.5,
        _           => 1.0,
    }
}

/// DEF multiplier
///   = (att_lv + 20) / ((def_lv + 20) * max(0, 1 - def_ignore - def_reduction) + (att_lv + 20))
fn def_mult(att_lv: i32, def_lv: i32, def_ignore: f64, def_reduction: f64) -> f64 {
    let def_factor = (1.0 - def_ignore - def_reduction).max(0.0);
    let att = (att_lv + 20) as f64;
    let def = (def_lv + 20) as f64;
    att / (def * def_factor + att)
}

/// RES multiplier = clamp(1 - (res - res_pen), 0.10, 2.00)
fn res_mult(res: f64, res_pen: f64) -> f64 {
    (1.0 - (res - res_pen)).clamp(0.10, 2.00)
}

/// Resolve the correct percent-bonus field for the scaling stat.
///
/// DEF-scaling chars (Aventurine) use `def_percent`.
/// HP-scaling chars use `hp_percent`.
/// Everything else (default ATK-scaling) uses `atk_percent`.
fn scaling_stat_percent(attacker: &TeamMember, scaling_stat_id: &str) -> f64 {
    if scaling_stat_id == ids::CHAR_DEF_ID {
        attacker.buffs.def_percent
    } else if scaling_stat_id == ids::CHAR_HP_ID {
        attacker.buffs.hp_percent
    } else {
        attacker.buffs.atk_percent
    }
}

/// Additional DMG% that applies only to specific action types.
fn action_type_dmg_boost(attacker: &TeamMember, action: &ActionParams) -> f64 {
    match action.action_type {
        ActionType::Basic    => attacker.buffs.basic_atk_dmg_boost,
        ActionType::Skill    => attacker.buffs.skill_dmg_boost,
        ActionType::Ultimate => attacker.buffs.ult_dmg_boost,
        ActionType::FollowUp => attacker.buffs.follow_up_dmg_boost,
        _ => 0.0,
    }
}

/// Intermediate values from the damage formula, for logging.
pub struct DamageComponents {
    pub char_base: f64,
    pub lc_base:   f64,
    pub total_stat: f64,
    pub base_dmg:   f64,
    pub dmg_boost:  f64,
    pub def_m:      f64,
    pub res_m:      f64,
    pub vuln_m:     f64,
    pub mitig_m:    f64,
    pub broken_m:   f64,
    pub crit_m:     f64,
}

/// Returns damage + all intermediate multipliers for debug logging.
pub fn calculate_damage_detailed(
    attacker: &TeamMember,
    target: &SimEnemy,
    action: &ActionParams,
) -> (f64, DamageComponents) {
    let char_base  = attacker.base_stats.get(&action.scaling_stat_id).copied().unwrap_or(1000.0);
    let lc_base    = attacker.lightcone.base_stats.get(&action.scaling_stat_id).copied().unwrap_or(0.0);
    let pct_bonus  = scaling_stat_percent(attacker, &action.scaling_stat_id);
    let total_stat = (char_base + lc_base) * (1.0 + pct_bonus / 100.0);
    let base_dmg   = (action.multiplier + action.extra_multiplier / 100.0) * total_stat + action.extra_dmg;
    let dmg_boost  = 1.0 + (attacker.buffs.dmg_boost + action_type_dmg_boost(attacker, action)) / 100.0;
    let weaken_m   = 1.0 - attacker.buffs.weaken / 100.0;
    let enemy_def_reduce: f64 = target.active_debuffs.values()
        .filter_map(|e| {
            let s = e.stat.as_deref().unwrap_or("").to_ascii_lowercase();
            if s == "def reduction" || s == "def shred" { Some(e.value) } else { None }
        }).sum();
    let def_m = def_mult(
        attacker.level, target.level,
        attacker.buffs.def_ignore / 100.0,
        (attacker.buffs.def_reduction + enemy_def_reduce) / 100.0,
    );
    let base_res = target.elemental_res.get(&attacker.element).copied().unwrap_or(target.resistance);
    let all_res_reduce: f64 = target.active_debuffs.values()
        .filter_map(|e| if e.stat.as_deref() == Some("All RES") { Some(e.value / 100.0) } else { None }).sum();
    let weak_res_reduce: f64 = if target.weaknesses.contains(&attacker.element) {
        target.active_debuffs.values()
            .filter_map(|e| if e.stat.as_deref() == Some("Weakness RES") { Some(e.value / 100.0) } else { None }).sum()
    } else { 0.0 };
    let res_m  = res_mult(base_res - all_res_reduce - weak_res_reduce, attacker.buffs.res_pen / 100.0);
    let buff_vuln: f64 = target.active_buffs.values()
        .filter_map(|e| if e.stat.as_deref() == Some("Vulnerability") { Some(e.value) } else { None }).sum();
    let vuln_m  = 1.0 + (target.vulnerability + buff_vuln) / 100.0;
    let mitig_m = 1.0 - target.dmg_reduction / 100.0;
    let broken_m = if target.is_broken { 1.0 } else { 0.9 };
    let cr       = (attacker.buffs.crit_rate / 100.0).clamp(0.0, 1.0);
    let cd       = attacker.buffs.crit_dmg / 100.0;
    let crit_m   = 1.0 + cr * cd;

    let final_dmg = (base_dmg * dmg_boost * weaken_m * def_m * res_m * vuln_m * mitig_m * broken_m * crit_m).floor();
    (final_dmg, DamageComponents { char_base, lc_base, total_stat, base_dmg, dmg_boost, def_m, res_m, vuln_m, mitig_m, broken_m, crit_m })
}

/// Full HSR damage formula (expected crit value).
///
/// DMG = BaseDMG × CritMult(expected) × DMGBoostMult × WeakenMult
///       × DEFMult × RESMult × VulnMult × MitigMult × BrokenMult
pub fn calculate_damage(attacker: &TeamMember, target: &SimEnemy, action: &ActionParams) -> f64 {
    // 1. Scaling stat total = (char_base + lc_base) × (1 + stat%)
    //    stat% is atk_percent for ATK-scaling, def_percent for DEF-scaling,
    //    hp_percent for HP-scaling characters.
    let char_base  = attacker.base_stats.get(&action.scaling_stat_id).copied().unwrap_or(1000.0);
    let lc_base    = attacker.lightcone.base_stats.get(&action.scaling_stat_id).copied().unwrap_or(0.0);
    let pct_bonus  = scaling_stat_percent(attacker, &action.scaling_stat_id);
    let total_stat = (char_base + lc_base) * (1.0 + pct_bonus / 100.0);

    // 2. BaseDMG = (multiplier + extra_mult%) × stat + extra_dmg
    let base_dmg = (action.multiplier + action.extra_multiplier / 100.0) * total_stat + action.extra_dmg;

    // 3. DMG Boost (all-damage % + action-type-specific %)
    let dmg_boost_mult = 1.0 + (attacker.buffs.dmg_boost + action_type_dmg_boost(attacker, action)) / 100.0;

    // 4. Weaken (attacker's outgoing penalty)
    let weaken_mult = 1.0 - attacker.buffs.weaken / 100.0;

    // 5. DEF — attacker-side ignore + enemy-side DEF reduction debuffs
    let enemy_def_reduce: f64 = target.active_debuffs.values()
        .filter_map(|e| {
            let s = e.stat.as_deref().unwrap_or("").to_ascii_lowercase();
            if s == "def reduction" || s == "def shred" { Some(e.value) } else { None }
        })
        .sum();
    let d_mult = def_mult(
        attacker.level,
        target.level,
        attacker.buffs.def_ignore / 100.0,
        (attacker.buffs.def_reduction + enemy_def_reduce) / 100.0,
    );

    // 6. RES — element-specific base, minus enemy All-RES / Weakness-RES debuffs
    let base_res = target.elemental_res
        .get(&attacker.element)
        .copied()
        .unwrap_or(target.resistance);
    let all_res_reduce: f64 = target.active_debuffs.values()
        .filter_map(|e| {
            if e.stat.as_deref() == Some("All RES") { Some(e.value / 100.0) } else { None }
        })
        .sum();
    let weak_res_reduce: f64 = if target.weaknesses.contains(&attacker.element) {
        target.active_debuffs.values()
            .filter_map(|e| {
                if e.stat.as_deref() == Some("Weakness RES") { Some(e.value / 100.0) } else { None }
            })
            .sum()
    } else {
        0.0
    };
    let r_mult = res_mult(
        base_res - all_res_reduce - weak_res_reduce,
        attacker.buffs.res_pen / 100.0,
    );

    // 7. Vulnerability — direct field + active_buffs tagged "Vulnerability"
    let buff_vuln: f64 = target.active_buffs.values()
        .filter_map(|e| {
            if e.stat.as_deref() == Some("Vulnerability") { Some(e.value) } else { None }
        })
        .sum();
    let vuln_mult = 1.0 + (target.vulnerability + buff_vuln) / 100.0;

    // 8. Mitigation
    let mitig_mult = 1.0 - target.dmg_reduction / 100.0;

    // 9. Broken
    let broken_mult = if target.is_broken { 1.0 } else { 0.9 };

    // 10. Expected CRIT
    let cr = (attacker.buffs.crit_rate / 100.0).clamp(0.0, 1.0);
    let cd = attacker.buffs.crit_dmg / 100.0;
    let expected_crit = 1.0 + cr * cd;

    (base_dmg * dmg_boost_mult * weaken_mult * d_mult * r_mult * vuln_mult * mitig_mult * broken_mult * expected_crit).floor()
}

/// Instant break damage dealt at the moment toughness hits 0.
///
/// BaseDMG = BREAK_COEFF[element] × LevelMult × MaxToughnessMult
/// DMG     = BaseDMG × (1 + BreakEffect%) × DEFMult × RESMult × VulnMult × MitigMult × 0.9
pub fn calculate_break_damage(attacker: &TeamMember, target: &SimEnemy) -> f64 {
    let lv_mult = level_mult(attacker.level);
    let max_tough_mult = 0.5 + target.max_toughness / 40.0;
    let base_dmg = break_base_coeff(&attacker.element) * lv_mult * max_tough_mult;

    let be = (attacker.base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0)
             + attacker.buffs.break_effect) / 100.0;

    let d_mult = def_mult(
        attacker.level,
        target.level,
        attacker.buffs.def_ignore / 100.0,
        attacker.buffs.def_reduction / 100.0,
    );
    let base_res = target.elemental_res
        .get(&attacker.element)
        .copied()
        .unwrap_or(target.resistance);
    let r_mult = res_mult(base_res, attacker.buffs.res_pen / 100.0);
    let vuln_mult  = 1.0 + target.vulnerability / 100.0;
    let mitig_mult = 1.0 - target.dmg_reduction / 100.0;
    let broken_mult = 0.9; // enemy was not yet broken when toughness hits 0

    (base_dmg * (1.0 + be) * d_mult * r_mult * vuln_mult * mitig_mult * broken_mult).floor()
}

/// Toughness reduction for an ability hit.
///
/// = (base + additive) × (1 + reduction_increase%) × (1 + break_efficiency%) × ability_mult
pub fn calculate_toughness_reduction(
    base: f64,
    additive: f64,
    reduction_increase: f64,
    break_efficiency: f64,
    ability_mult: f64,
) -> f64 {
    let capped_efficiency = break_efficiency.min(300.0);
    (base + additive)
        * (1.0 + reduction_increase / 100.0)
        * (1.0 + capped_efficiency / 100.0)
        * ability_mult
}
