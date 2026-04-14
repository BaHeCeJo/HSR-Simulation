use crate::models::*;
use std::collections::HashMap;
use uuid::Uuid;

pub const CHAR_SPD_ID: &str = "3e4b082d-7943-440d-ae2c-8d31b0a370be";
pub const ATK_ID: &str = "c987f652-6a0b-487f-9e4b-af2c9b51c6aa";
pub const CRIT_RATE_ID: &str = "a62e3a38-743a-41f8-8523-aec4ef998c84";
pub const CRIT_DMG_ID: &str = "a93e523a-7852-4580-b2ef-03467e214bcd";

pub const ENEMY_SPD_ID: &str = "b0bfd27b-0a5f-4329-a280-dc1c998446cb";

pub struct SimState {
    pub team: Vec<Character>,
    pub enemies: Vec<Option<Enemy>>,
    pub current_av: f64,
    pub max_av: f64,
    pub total_damage: f64,
    pub logs: Vec<String>,
    pub av_queue: Vec<ActorInfo>,
    pub skill_points: i32,
}

#[derive(Clone)]
pub struct ActorInfo {
    pub id: Uuid,
    pub instance_id: String,
    pub next_av: f64,
    pub is_enemy: bool,
}

pub fn run_simulation(team: Vec<Character>, waves: Vec<Wave>, settings: Settings) -> SimReport {
    let max_av = 150.0 + (settings.max_cycles - 1) as f64 * 100.0;
    let mut state = SimState {
        team: team.clone(),
        enemies: waves[0].enemies.clone(),
        current_av: 0.0,
        max_av,
        total_damage: 0.0,
        logs: Vec::new(),
        av_queue: Vec::new(),
        skill_points: 3,
    };

    let atk_uuid = Uuid::parse_str(ATK_ID).unwrap();
    let spd_uuid = Uuid::parse_str(CHAR_SPD_ID).unwrap();
    let enemy_spd_uuid = Uuid::parse_str(ENEMY_SPD_ID).unwrap();

    // Initialize AV Queue
    for char in &state.team {
        let spd = char.basic_stats.get(&spd_uuid).map(|s| s.value).unwrap_or(100.0);
        state.av_queue.push(ActorInfo {
            id: char.character_id,
            instance_id: String::new(),
            next_av: 10000.0 / spd,
            is_enemy: false,
        });
    }

    for enemy_opt in &state.enemies {
        if let Some(enemy) = enemy_opt {
            let spd = enemy.basic_stats.get(&enemy_spd_uuid).map(|s| s.value).unwrap_or(100.0);
            state.av_queue.push(ActorInfo {
                id: enemy.id,
                instance_id: enemy.instance_id.clone(),
                next_av: 10000.0 / spd,
                is_enemy: true,
            });
        }
    }

    while state.current_av <= state.max_av {
        state.av_queue.sort_by(|a, b| a.next_av.partial_cmp(&b.next_av).unwrap());
        let actor_info = state.av_queue[0].clone();
        state.current_av = actor_info.next_av;

        if state.current_av > state.max_av {
            break;
        }

        if !actor_info.is_enemy {
            let char_idx = state.team.iter().position(|c| c.character_id == actor_info.id).unwrap();
            let character = &state.team[char_idx];
            
            // Basic Attack Logic
            let mut multiplier = 1.0;
            if let Some(basic_ability) = character.abilities.iter().find(|a| a.name.to_lowercase().contains("basic") || a.name.contains("Wiltcross")) {
                if let Some(scaling) = basic_ability.scalings.iter().find(|s| s.scaling_stat_id == Some(atk_uuid)) {
                    multiplier = scaling.value / 100.0;
                }
            }

            if let Some(enemy) = state.enemies.iter().flatten().next() {
                let damage = calculate_damage(character, enemy, multiplier, &atk_uuid);
                state.total_damage += damage;
            }
            
            let spd = character.basic_stats.get(&spd_uuid).map(|s| s.value).unwrap_or(100.0);
            state.av_queue[0].next_av += 10000.0 / spd;
        } else {
            let enemy_idx = state.enemies.iter().position(|e| e.as_ref().map_or(false, |ee| ee.instance_id == actor_info.instance_id)).unwrap();
            let enemy = state.enemies[enemy_idx].as_ref().unwrap();
            let spd = enemy.basic_stats.get(&enemy_spd_uuid).map(|s| s.value).unwrap_or(100.0);
            state.av_queue[0].next_av += 10000.0 / spd;
        }
    }

    SimReport {
        total_damage: state.total_damage,
        cycles_taken: settings.max_cycles,
        logs: state.logs,
        is_defeated: false,
    }
}

fn calculate_damage(char: &Character, enemy: &Enemy, multiplier: f64, atk_uuid: &Uuid) -> f64 {
    let atk = char.basic_stats.get(atk_uuid).map(|s| s.value).unwrap_or(1000.0);
    let base_dmg = atk * multiplier;
    
    let char_level = char.level as f64;
    let enemy_level = enemy.level as f64;
    let def_mult = (char_level + 20.0) / ((enemy_level + 20.0) + (char_level + 20.0));
    
    let res_mult = 0.8;
    let dmg_boost = 1.0;
    
    let cr_uuid = Uuid::parse_str(CRIT_RATE_ID).unwrap();
    let cd_uuid = Uuid::parse_str(CRIT_DMG_ID).unwrap();
    let cr = char.advanced_stats.get(&cr_uuid).map(|s| s.value).unwrap_or(5.0) / 100.0;
    let cd = char.advanced_stats.get(&cd_uuid).map(|s| s.value).unwrap_or(50.0) / 100.0;
    let crit_mult = 1.0 + (cr.max(0.0).min(1.0) * cd);
    
    base_dmg * def_mult * res_mult * dmg_boost * crit_mult
}
