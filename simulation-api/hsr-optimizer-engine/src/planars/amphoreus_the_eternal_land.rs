//! Amphoreus, The Eternal Land
//!
//! (2p) CRIT Rate +8%.
//!      While the wearer's memosprite is on the field, all allies' SPD +8%.
//!      Cannot stack.
//!      The team SPD +8% only applies if the wearer has a memosprite (has_memo = true).

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "amphoreus_the_eternal_land";

pub fn apply(member: &mut TeamMember) {
    member.buffs.crit_rate += 8.0;
}

/// Team bonus: if any memo-character wears this set, all allies gain SPD +8%.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_memo_wearer = team.iter().zip(relic_lists.iter())
        .any(|(m, r)| m.has_memo && r.iter().any(|p| p.set_id == SET_ID));
    if any_memo_wearer {
        for member in team.iter_mut() {
            member.buffs.speed_percent += 8.0;
        }
    }
}
