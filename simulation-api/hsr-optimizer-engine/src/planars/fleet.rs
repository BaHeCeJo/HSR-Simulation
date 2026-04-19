//! Fleet of the Ageless
//!
//! (2p) Wearer Max HP +12%.
//!      When wearer SPD >= 120, all allies ATK +8%.

use crate::ids;
use crate::models::TeamMember;

const SET_ID: &str = "fleet_of_the_ageless";

/// Per-wearer bonus: HP +12%.
pub fn apply_per_wearer(member: &mut TeamMember) {
    member.buffs.hp_percent += 12.0;
}

/// Team bonus: if any Fleet wearer has SPD >= 120, all allies gain ATK +8%.
///
/// Call AFTER per-character bonuses (including Musketeer SPD%) have been applied
/// so the SPD threshold uses the correct final value.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<crate::models::IncomingRelic>]) {
    let any_high_spd = (0..team.len()).any(|i| {
        if !relic_lists[i].iter().any(|r| r.set_id == SET_ID) {
            return false;
        }
        let base = team[i].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        let spd  = base * (1.0 + team[i].buffs.speed_percent / 100.0);
        spd >= 120.0
    });

    if any_high_spd {
        for member in team.iter_mut() {
            member.buffs.atk_percent += 8.0;
        }
    }
}
