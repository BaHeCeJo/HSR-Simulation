#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::cmp::Ordering;

// ─── Stat map: UUID string → float value ────────────────────────────────────
pub type StatMap = HashMap<String, f64>;

// ─── Incoming API types (match TypeScript server.ts) ────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatValue {
    pub value: f64,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingScaling {
    pub level: i32,
    pub value: f64,
    pub value_type: Option<String>,
    pub scaling_stat_id: Option<String>,
    pub attribute_index: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingAbility {
    pub name: Option<String>,
    pub level: Option<i32>,
    pub scalings: Option<Vec<IncomingScaling>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingCharacter {
    pub character_id: String,
    pub name: Option<String>,
    pub level: Option<i32>,
    pub eidolon: Option<i32>,
    pub attribute: Option<String>,  // element (e.g. "Lightning")
    pub path: Option<String>,
    pub basic_stats: Option<HashMap<String, StatValue>>,
    pub advanced_stats: Option<HashMap<String, StatValue>>,
    pub abilities: Option<Vec<IncomingAbility>>,
    /// Up to 6 equipped relic/ornament pieces (4 relic + 2 ornament slots).
    pub relics: Option<Vec<IncomingRelic>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingLightcone {
    pub lightcone_id: Option<String>,
    pub name: Option<String>,
    pub level: Option<i32>,
    pub superimposition: Option<i32>,
    pub path: Option<String>,
    pub basic_stats: Option<HashMap<String, StatValue>>,
    pub advanced_stats: Option<HashMap<String, StatValue>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingEnemy {
    pub id: String,
    pub instance_id: String,
    pub name: Option<String>,
    pub level: Option<i32>,
    pub basic_stats: Option<HashMap<String, StatValue>>,
    pub advanced_stats: Option<HashMap<String, StatValue>>,
    pub resistances: Option<HashMap<String, f64>>,
    pub weaknesses: Option<Vec<String>>,
    pub tier: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingWave {
    pub enemies: Option<Vec<Option<IncomingEnemy>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub max_cycles: Option<i32>,
    pub has_castorice: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizePayload {
    pub character_pool: Vec<IncomingCharacter>,
    pub lightcone_pool: Option<Vec<IncomingLightcone>>,
    pub waves: Vec<IncomingWave>,
    pub settings: Option<Settings>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizeRequest {
    pub command: Option<String>,
    pub game: Option<String>,
    pub payload: OptimizePayload,
}

// ─── Simulation output types ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub av: f64,
    pub actor: String,
    pub message: String,
    pub sub_entries: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimReport {
    pub total_damage: f64,
    pub cycles_taken: i32,
    pub logs: Vec<LogEntry>,
    pub is_defeated: bool,
}

/// Per-character relic configuration returned in the optimize response.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CharRelicConfig {
    pub character_name: String,
    pub relic_set:      String,   // e.g. "Musketeer 4p" / "No Relic Set"
    pub ornament_set:   String,   // e.g. "Fleet 2p" / "No Ornament Set"
    pub body_main:      String,
    pub feet_main:      String,
    pub sphere_main:    String,
    pub rope_main:      String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeResult {
    pub best_team: Vec<String>,
    pub total_damage: f64,
    pub cycles: i32,
    pub logs: Vec<String>,
    pub simulations_count: usize,
    pub is_defeated: bool,
    pub best_relics: Vec<CharRelicConfig>,
}

// ─── Internal simulation types ───────────────────────────────────────────────

// ─── Relic / Ornament types ──────────────────────────────────────────────────

/// The six equipment slots (4 Relic + 2 Ornament).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelicSlot {
    Head,
    Hands,
    Body,
    Feet,
    PlanarSphere,
    LinkRope,
}

/// One equipped relic/ornament piece as sent by the TypeScript server.
/// Substats will be added in a later iteration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingRelic {
    pub set_id:    String,    // e.g. "musketeer_of_wild_wheat"
    pub slot:      RelicSlot,
    /// Main stat key — one of the values defined in relics.rs::MAIN_STAT_*
    pub main_stat: String,
}

// ─── Buffs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Buffs {
    // ── Scaling stat percentage bonuses (applied inside damage formula) ──────
    pub atk_percent: f64,          // ATK%   (fleet buff, LC passives, etc.)
    pub def_percent: f64,          // DEF%   (Aventurine traces, relic DEF%)
    pub hp_percent: f64,           // HP%    (relic HP%, fleet 2-piece)
    pub speed_percent: f64,        // SPD%   (Musketeer 4-piece)
    // ── Combat multipliers ───────────────────────────────────────────────────
    pub crit_rate: f64,            // base 5%
    pub crit_dmg: f64,             // base 50%
    pub dmg_boost: f64,            // all-damage %
    pub basic_atk_dmg_boost: f64,  // Basic ATK only % (Musketeer 4-piece)
    pub skill_dmg_boost: f64,      // Skill only % (Firesmith 4p, Scholar 4p)
    pub ult_dmg_boost: f64,        // Ultimate only % (Wind-Soaring 4p, Scholar 4p)
    pub follow_up_dmg_boost: f64,  // Follow-up only % (Ashblazing Grand Duke 2p)
    pub def_ignore: f64,
    pub def_reduction: f64,
    pub extra_multiplier: f64,
    pub extra_dmg: f64,
    pub res_pen: f64,
    pub weaken: f64,
    pub break_efficiency: f64,
    /// Temporary Break Effect % bonus (Watchmaker 4p, etc.) — added to base_stats BE in break calc.
    pub break_effect: f64,
    // ── Utility stats ────────────────────────────────────────────────────────
    pub outgoing_healing: f64,
    pub effect_hit_rate: f64,
    /// Reduces the chance of incoming debuffs landing on this character.
    /// Formula: RealChance = BaseChance × (1 + EHR_attacker) × (1 − EffectRES) × (1 − DebuffRES)
    pub effect_res: f64,
    pub energy_regen_rate: f64,
    /// % reduction to all incoming damage (Guard 2p, etc.). Applied before shield absorption.
    pub incoming_dmg_reduction: f64,
    /// % boost to shield size when the wearer creates a shield (Knight 4p, etc.).
    pub shield_effect: f64,
}

impl Default for Buffs {
    fn default() -> Self {
        Buffs {
            atk_percent:         0.0,
            def_percent:         0.0,
            hp_percent:          0.0,
            speed_percent:       0.0,
            crit_rate:           5.0,
            crit_dmg:           50.0,
            dmg_boost:           0.0,
            basic_atk_dmg_boost: 0.0,
            skill_dmg_boost:     0.0,
            ult_dmg_boost:       0.0,
            follow_up_dmg_boost: 0.0,
            def_ignore:          0.0,
            def_reduction:       0.0,
            extra_multiplier:    0.0,
            extra_dmg:           0.0,
            res_pen:             0.0,
            weaken:              0.0,
            break_efficiency:    0.0,
            break_effect:        0.0,
            outgoing_healing:         0.0,
            effect_hit_rate:          0.0,
            effect_res:               0.0,
            energy_regen_rate:        0.0,
            incoming_dmg_reduction:   0.0,
            shield_effect:            0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatChange {
    pub stat: String,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct StatusEffect {
    pub duration: i32,
    pub value: f64,
    pub stat: Option<String>,
    pub effects: Vec<StatChange>,
}

#[derive(Debug, Clone)]
pub struct LightconeStats {
    pub base_stats: StatMap,
    pub scaling: f64,
    /// Lightcone ID string from the payload (empty string if no LC equipped).
    pub id: String,
    /// Superimposition level 1–5.
    pub superimposition: i32,
}

#[derive(Debug, Clone)]
pub struct TeamMember {
    pub kit_id: String,
    pub name: String,
    pub element: String,
    pub path: String,
    pub level: i32,
    pub eidolon: i32,
    pub hp: f64,
    pub max_hp: f64,
    pub shield: f64,
    pub is_downed: bool,
    pub toughness: f64,
    pub max_toughness: f64,
    pub is_broken: bool,
    pub energy: f64,
    pub max_energy: f64,
    pub ability_levels: AbilityLevels,
    pub base_stats: StatMap,
    pub buffs: Buffs,
    pub active_buffs: HashMap<String, StatusEffect>,
    pub active_debuffs: HashMap<String, StatusEffect>,
    pub lightcone: LightconeStats,
    pub stacks: HashMap<&'static str, f64>,
    pub turn_counters: HashMap<&'static str, i32>,
    pub aggro_modifier: f64,
    /// Raw ability data from the database (for scaling lookups)
    pub abilities: Vec<IncomingAbility>,
    /// Equipped relic/ornament pieces (main stats already baked into buffs/base_stats).
    /// Kept here so set-bonus logic can count pieces per set.
    pub relics: Vec<IncomingRelic>,
    /// True for Remembrance-path characters that deploy a persistent memosprite/summon.
    /// Gates memo-conditional set bonuses (World-Remaking Deliverer, Amphoreus, etc.).
    pub has_memo: bool,
    /// True for characters whose kit revolves around follow-up attacks.
    /// Gates FUA-conditional set bonuses (Ashblazing, Wind-Soaring Valorous, Duran, etc.).
    pub is_fua: bool,
}

#[derive(Debug, Clone)]
pub struct AbilityLevels {
    pub basic: i32,
    pub skill: i32,
    pub ultimate: i32,
    pub talent: i32,
}

impl Default for AbilityLevels {
    fn default() -> Self {
        AbilityLevels { basic: 6, skill: 10, ultimate: 10, talent: 10 }
    }
}

#[derive(Debug, Clone)]
pub struct SimEnemy {
    pub kit_id: String,
    pub instance_id: String,
    pub name: String,
    pub level: i32,
    pub hp: f64,
    pub max_hp: f64,
    pub toughness: f64,
    pub max_toughness: f64,
    pub is_broken: bool,
    pub weaknesses: Vec<String>,
    pub resistance: f64,
    pub elemental_res: HashMap<String, f64>,
    pub vulnerability: f64,
    pub dmg_reduction: f64,
    pub weaken: f64,
    pub debuff_count: u32,
    pub effect_res: f64,
    pub tier: String,
    pub active_debuffs: HashMap<String, StatusEffect>,
    pub active_buffs: HashMap<String, StatusEffect>,
    pub base_stats: StatMap,
    /// Cached sums recomputed whenever debuffs/buffs change — read directly in damage.rs
    pub cached_def_reduce: f64,
    pub cached_all_res_reduce: f64,
    pub cached_weakness_res_reduce: f64,
    pub cached_vuln_bonus: f64,
}

#[derive(Debug, Clone)]
pub struct Wave {
    pub initial_enemies: Vec<Option<SimEnemy>>,
    pub enemy_pool: Vec<SimEnemy>,
}

// ─── AV Queue entry (min-heap by next_av) ────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActorEntry {
    /// AV at which this actor takes their next turn
    pub next_av: f64,
    /// Character kit_id or enemy kit_id
    pub actor_id: String,
    /// Instance id (distinguishes multiple enemies of same type)
    pub instance_id: String,
    pub is_enemy: bool,
}

impl PartialEq for ActorEntry {
    fn eq(&self, other: &Self) -> bool {
        self.next_av == other.next_av
            && self.actor_id == other.actor_id
            && self.instance_id == other.instance_id
    }
}

impl Eq for ActorEntry {}

impl PartialOrd for ActorEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ActorEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse so that BinaryHeap (max-heap) becomes a min-heap on next_av
        other.next_av.partial_cmp(&self.next_av)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.actor_id.cmp(&other.actor_id))
    }
}

// ─── Action info passed to hooks and damage calc ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionType {
    Basic,
    Skill,
    Ultimate,
    FollowUp,
    TalentProc,
    EnemyAttack,
}

#[derive(Debug, Clone)]
pub struct ActionParams {
    pub action_type: ActionType,
    pub scaling_stat_id: String,
    pub multiplier: f64,
    pub extra_multiplier: f64,
    pub extra_dmg: f64,
    pub toughness_damage: f64,
    /// Set true in on_before_action to trigger on_global_debuff dispatch after this action
    pub inflicts_debuff: bool,
    /// Set true when this action counts as ultimate DMG (affects zone vulnerability checks)
    pub is_ult_dmg: bool,
}

// ─── SimState (the full mutable combat state) ────────────────────────────────

use std::collections::BinaryHeap;

pub struct SimState {
    pub team: Vec<TeamMember>,
    pub enemies: Vec<Option<SimEnemy>>,
    pub waves: Vec<Wave>,
    pub current_wave_index: usize,
    pub av_queue: BinaryHeap<ActorEntry>,
    pub current_av: f64,
    pub max_av: f64,
    pub skill_points: i32,
    pub total_damage: f64,
    pub logs: Vec<LogEntry>,
    pub nihility_count: i32,
    pub with_logs: bool,
    /// Global cross-character/cross-entity state (Ashen Roast stacks, zone counters, etc.)
    pub stacks: HashMap<String, f64>,
    /// Incremented each action; used by characters to detect duplicate debuff triggers
    pub current_action_id: u64,
}

impl SimState {
    pub fn add_log(&mut self, actor: &str, message: String) {
        if self.with_logs {
            self.logs.push(LogEntry {
                av: self.current_av,
                actor: actor.to_string(),
                message,
                sub_entries: Vec::new(),
            });
        }
    }

    pub fn add_log_sub(&mut self, sub: String) {
        if self.with_logs {
            if let Some(last) = self.logs.last_mut() {
                last.sub_entries.push(sub);
            }
        }
    }

    /// Returns the index in `team` for the given kit_id, or None.
    pub fn find_member_idx(&self, kit_id: &str) -> Option<usize> {
        self.team.iter().position(|m| m.kit_id == kit_id)
    }

    /// Returns the index in `enemies` for the given instance_id, or None.
    pub fn find_enemy_idx(&self, instance_id: &str) -> Option<usize> {
        self.enemies.iter().position(|e| {
            e.as_ref().map_or(false, |e| e.instance_id == instance_id)
        })
    }

    /// True when every slot in the current enemy list is None (all dead).
    pub fn all_enemies_dead(&self) -> bool {
        self.enemies.iter().all(|e| e.is_none())
    }

    /// Count living team members.
    pub fn living_count(&self) -> usize {
        self.team.iter().filter(|m| !m.is_downed).count()
    }

    /// Apply damage to the first living enemy (simple targeting for skeleton).
    pub fn apply_damage_to_first_enemy(&mut self, damage: f64, toughness_damage: f64, element: &str) {
        for slot in self.enemies.iter_mut() {
            if let Some(enemy) = slot {
                enemy.hp -= damage;
                self.total_damage += damage;

                // Toughness
                if !enemy.is_broken {
                    let toughness_dmg = if enemy.weaknesses.contains(&element.to_string()) {
                        toughness_damage
                    } else {
                        0.0
                    };
                    enemy.toughness -= toughness_dmg;
                    if enemy.toughness <= 0.0 {
                        enemy.toughness = 0.0;
                        enemy.is_broken = true;
                    }
                }

                // Remove dead enemy
                if enemy.hp <= 0.0 {
                    *slot = None;
                }
                return;
            }
        }
    }
}
