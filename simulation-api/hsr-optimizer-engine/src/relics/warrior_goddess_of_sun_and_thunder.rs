//! Warrior Goddess of Sun and Thunder
//!
//! (2p) SPD +6%.
//! (4p) When wearer and memosprite provide healing to allies (not self), wearer gains
//!      "Gentle Rain" (1× per turn, 2 turns).  While active: wearer SPD +6%, all allies
//!      CRIT DMG +15%.
//!      4p only applies when the wearer has a memosprite (has_memo = true).

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "warrior_goddess_of_sun_and_thunder";

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.speed_percent += 6.0;
    }
    if count >= 4 && member.has_memo {
        member.buffs.speed_percent += 6.0; // wearer SPD while Gentle Rain active
    }
}

/// Team bonus: if any memo-character wears 4× Warrior Goddess, all allies gain CRIT DMG +15%.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_memo_4p = team.iter().zip(relic_lists.iter()).any(|(m, r)| {
        m.has_memo && r.iter().filter(|p| p.set_id == SET_ID).count() >= 4
    });
    if any_memo_4p {
        for member in team.iter_mut() {
            member.buffs.crit_dmg += 15.0;
        }
    }
}
