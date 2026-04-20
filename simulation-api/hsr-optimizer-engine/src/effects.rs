#![allow(dead_code)]
use std::collections::HashMap;
use crate::models::{SimEnemy, StatusEffect, TeamMember};

/// Tick all active buffs on a team member: decrement durations, remove expired.
pub fn tick_buffs(member: &mut TeamMember) {
    member.active_buffs.retain(|_, effect| {
        effect.duration -= 1;
        effect.duration > 0
    });
}

/// Tick all active debuffs on a team member.
pub fn tick_debuffs(member: &mut TeamMember) {
    member.active_debuffs.retain(|_, effect| {
        effect.duration -= 1;
        effect.duration > 0
    });
}

/// Recompute all damage-formula cache fields from current debuff/buff state.
/// Call after any change to active_debuffs or active_buffs on an enemy.
pub fn recompute_enemy_caches(enemy: &mut SimEnemy) {
    enemy.cached_def_reduce = enemy.active_debuffs.values()
        .filter_map(|e| {
            let s = e.stat.as_deref().unwrap_or("").to_ascii_lowercase();
            if s == "def reduction" || s == "def shred" { Some(e.value) } else { None }
        })
        .sum();
    enemy.cached_all_res_reduce = enemy.active_debuffs.values()
        .filter_map(|e| if e.stat.as_deref() == Some("All RES") { Some(e.value / 100.0) } else { None })
        .sum();
    enemy.cached_weakness_res_reduce = enemy.active_debuffs.values()
        .filter_map(|e| if e.stat.as_deref() == Some("Weakness RES") { Some(e.value / 100.0) } else { None })
        .sum();
    enemy.cached_vuln_bonus = enemy.active_buffs.values()
        .filter_map(|e| if e.stat.as_deref() == Some("Vulnerability") { Some(e.value) } else { None })
        .sum();
}

/// Tick all active debuffs on an enemy.
pub fn tick_enemy_debuffs(enemy: &mut SimEnemy) {
    let before = enemy.active_debuffs.len();
    enemy.active_debuffs.retain(|_, effect| {
        effect.duration -= 1;
        effect.duration > 0
    });
    let removed = before - enemy.active_debuffs.len();
    enemy.debuff_count = enemy.debuff_count.saturating_sub(removed as u32);
    if removed > 0 {
        recompute_enemy_caches(enemy);
    }
}

/// Apply or overwrite a status effect by key (longer duration wins).
pub fn apply_status_effect(
    target: &mut HashMap<String, StatusEffect>,
    key: &str,
    effect: StatusEffect,
) {
    let entry = target.entry(key.to_string()).or_insert_with(|| effect.clone());
    if effect.duration > entry.duration {
        *entry = effect;
    }
}

/// Apply a buff to an enemy (e.g. vulnerability stacks).
pub fn apply_enemy_buff(enemy: &mut SimEnemy, key: &str, effect: StatusEffect) {
    apply_status_effect(&mut enemy.active_buffs, key, effect);
    recompute_enemy_caches(enemy);
}

/// Returns true if a debuff with `base_chance` (0.0–1.0) lands on `enemy`
/// given the attacker's combined Effect Hit Rate (as a percentage, e.g. 18.0 for 18%).
/// Formula: base_chance × (1 + EHR/100) × (1 − EffectRES/100) ≥ 1.0
pub fn debuff_lands(attacker_ehr: f64, enemy_effect_res: f64, base_chance: f64) -> bool {
    base_chance * (1.0 + attacker_ehr / 100.0) * (1.0 - enemy_effect_res / 100.0) >= 1.0
}

/// Apply a debuff only if it passes the landing check. Returns true if applied.
pub fn try_apply_enemy_debuff(
    attacker_ehr: f64,
    enemy: &mut SimEnemy,
    key: &str,
    effect: StatusEffect,
    base_chance: f64,
) -> bool {
    if !debuff_lands(attacker_ehr, enemy.effect_res, base_chance) {
        return false;
    }
    apply_enemy_debuff(enemy, key, effect);
    true
}

/// Apply a debuff to an enemy, incrementing its debuff counter if new.
pub fn apply_enemy_debuff(enemy: &mut SimEnemy, key: &str, effect: StatusEffect) {
    let is_new = !enemy.active_debuffs.contains_key(key);
    apply_status_effect(&mut enemy.active_debuffs, key, effect);
    if is_new {
        enemy.debuff_count += 1;
    }
    recompute_enemy_caches(enemy);
}

/// Apply a buff to a team member.
pub fn apply_member_buff(member: &mut TeamMember, key: &str, effect: StatusEffect) {
    apply_status_effect(&mut member.active_buffs, key, effect);
}

/// Compute the current effective value of a stat from active buffs.
/// Looks for effects whose `stat` field matches `stat_name`, sums additive values.
pub fn stat_from_buffs(buffs: &HashMap<String, StatusEffect>, stat_name: &str) -> f64 {
    buffs.values()
        .filter_map(|e| {
            // Check primary stat
            if e.stat.as_deref() == Some(stat_name) {
                return Some(e.value);
            }
            // Check multi-stat effects
            let sum: f64 = e.effects.iter()
                .filter(|sc| sc.stat == stat_name)
                .map(|sc| sc.value)
                .sum();
            if sum != 0.0 { Some(sum) } else { None }
        })
        .sum()
}
