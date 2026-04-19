//! Lushaka, the Sunken Seas
//!
//! (2p) Energy Regeneration Rate +5%.
//!      If the wearer is NOT the first character in the team lineup, the first
//!      character's ATK increases by 12%.
//!      Applied as a team bonus in `apply_team`.

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "lushaka_the_sunken_seas";

pub fn apply(member: &mut TeamMember) {
    member.buffs.energy_regen_rate += 5.0;
}

/// Team bonus: if any non-first member wears this set, the first character
/// in the lineup gains ATK +12%.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_non_first_wearer = relic_lists.iter().enumerate().any(|(i, r)| {
        i > 0 && r.iter().any(|p| p.set_id == SET_ID)
    });
    if any_non_first_wearer && !team.is_empty() {
        team[0].buffs.atk_percent += 12.0;
    }
}
