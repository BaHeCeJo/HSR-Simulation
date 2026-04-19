//! Izumo Gensei and Takama Divine Realm
//!
//! (2p) ATK +12% (guaranteed).
//!      When entering battle, if at least one other ally follows the same Path,
//!      CRIT Rate +12% — applied in `apply_team` with a real same-Path check.

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "izumo_gensei_and_takama_divine_realm";

pub fn apply(member: &mut TeamMember) {
    member.buffs.atk_percent += 12.0;
    // CRIT Rate +12% is handled in apply_team after verifying a same-Path ally exists.
}

/// Team bonus: each Izumo wearer gains CRIT Rate +12% if at least one other ally
/// shares the same Path.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let n = team.len();
    for i in 0..n {
        let has_izumo = relic_lists[i].iter().any(|p| p.set_id == SET_ID);
        if !has_izumo { continue; }
        let wearer_path = team[i].path.clone();
        let same_path_ally = (0..n)
            .filter(|&j| j != i)
            .any(|j| team[j].path == wearer_path);
        if same_path_ally {
            team[i].buffs.crit_rate += 12.0;
        }
    }
}
