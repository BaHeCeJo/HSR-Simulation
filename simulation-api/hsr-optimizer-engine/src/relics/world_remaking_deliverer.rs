//! World-Remaking Deliverer
//!
//! (2p) CRIT Rate +8%.
//! (4p) After the wearer uses Basic ATK or Skill (with memosprite on field),
//!      all allies' DMG +15% until the wearer's next Basic ATK or Skill.
//!      4p team DMG +15% only applies when the wearer has a memosprite (has_memo = true).

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "world_remaking_deliverer";

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.crit_rate += 8.0;
    }
    // 4p team bonus handled in apply_team.
}

/// Team bonus: if any memo-character wears 4× World-Remaking Deliverer,
/// all allies gain DMG +15%.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_memo_4p = team.iter().zip(relic_lists.iter()).any(|(m, r)| {
        m.has_memo && r.iter().filter(|p| p.set_id == SET_ID).count() >= 4
    });
    if any_memo_4p {
        for member in team.iter_mut() {
            member.buffs.dmg_boost += 15.0;
        }
    }
}
