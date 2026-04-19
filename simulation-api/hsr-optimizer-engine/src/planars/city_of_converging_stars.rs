//! City of Converging Stars
//!
//! (2p) When the wearer uses Follow-Up ATK, ATK +24% for 2 turns.
//!      When an enemy target gets defeated, all allies gain CRIT DMG +12%
//!      for the rest of the battle (cannot stack).
//!
//!      Per-wearer ATK only applies for FUA characters (is_fua = true).
//!      Team CRIT DMG +12% applied via `apply_team` (kill-triggered, always in multi-wave).

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "city_of_converging_stars";

pub fn apply(member: &mut TeamMember) {
    if member.is_fua {
        member.buffs.atk_percent += 24.0; // FUA chars trigger this frequently
    }
}

/// Team bonus: if any FUA-character wears this set, all allies gain CRIT DMG +12%
/// (kill condition is reliably met in any multi-enemy fight).
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_fua_wearer = team.iter().zip(relic_lists.iter())
        .any(|(m, r)| m.is_fua && r.iter().any(|p| p.set_id == SET_ID));
    if any_fua_wearer {
        for member in team.iter_mut() {
            member.buffs.crit_dmg += 12.0;
        }
    }
}
