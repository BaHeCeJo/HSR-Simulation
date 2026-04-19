//! Penacony, Land of the Dreams
//!
//! (2p) Energy Regeneration Rate +5%.
//!      All other allies that share the same DMG type as the wearer gain DMG +10%.
//!      Applied as a team bonus in `apply_team`.

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "penacony_land_of_the_dreams";

pub fn apply(member: &mut TeamMember) {
    member.buffs.energy_regen_rate += 5.0;
}

/// Team bonus: each wearer buffs all other allies that share their element with DMG +10%.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    // Collect (index, element) of all wearers first to avoid borrow conflicts.
    let wearers: Vec<(usize, String)> = relic_lists.iter().enumerate()
        .filter(|(_, r)| r.iter().any(|p| p.set_id == SET_ID))
        .map(|(i, _)| (i, team[i].element.clone()))
        .collect();

    for (wearer_idx, element) in wearers {
        for (j, member) in team.iter_mut().enumerate() {
            if j != wearer_idx && member.element == element {
                member.buffs.dmg_boost += 10.0;
            }
        }
    }
}
