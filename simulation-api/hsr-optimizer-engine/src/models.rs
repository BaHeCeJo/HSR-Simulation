use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatValue {
    pub value: f64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scaling {
    pub level: i32,
    pub value: f64,
    pub value_type: String, // "percent" or "flat"
    pub scaling_stat_id: Option<Uuid>,
    pub attribute_index: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ability {
    pub name: String,
    pub level: i32,
    pub scalings: Vec<Scaling>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Character {
    pub character_id: Uuid,
    pub name: String,
    pub level: i32,
    pub eidolon: i32,
    pub basic_stats: HashMap<Uuid, StatValue>,
    pub advanced_stats: HashMap<Uuid, StatValue>,
    pub abilities: Vec<Ability>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Lightcone {
    pub lightcone_id: Uuid,
    pub name: String,
    pub level: i32,
    pub superimposition: i32,
    pub basic_stats: HashMap<Uuid, StatValue>,
    pub advanced_stats: HashMap<Uuid, StatValue>,
    pub ability: Option<Ability>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Enemy {
    pub id: Uuid,
    pub instance_id: String,
    pub name: String,
    pub level: i32,
    pub basic_stats: HashMap<Uuid, StatValue>,
    pub advanced_stats: HashMap<Uuid, StatValue>,
    pub resistances: HashMap<String, f64>,
    pub weaknesses: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Wave {
    pub enemies: Vec<Option<Enemy>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub max_cycles: i32,
    pub has_castorice: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizePayload {
    pub character_pool: Vec<Character>,
    pub lightcone_pool: Vec<Lightcone>,
    pub waves: Vec<Wave>,
    pub settings: Settings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizeRequest {
    pub command: String,
    pub game: String,
    pub payload: OptimizePayload,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimReport {
    pub total_damage: f64,
    pub cycles_taken: i32,
    pub logs: Vec<String>,
    pub is_defeated: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizeResult {
    pub best_team: Vec<String>,
    pub total_damage: f64,
    pub cycles: i32,
    pub simulations_count: usize,
    pub is_defeated: bool,
}
