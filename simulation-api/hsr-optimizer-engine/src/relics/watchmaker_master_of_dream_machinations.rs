//! Watchmaker, Master of Dream Machinations
//!
//! (2p) Break Effect +16%.
//! (4p) When the wearer uses Ultimate on an ally, all allies' Break Effect +30%
//!      for 2 turns. Cannot stack.
//!      Approximated as 50% uptime → all allies +15% Break Effect via `apply_team`.

use crate::ids;
use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "watchmaker_master_of_dream_machinations";

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        *member.base_stats.entry(ids::CHAR_BE_ID.to_string()).or_insert(0.0) += 16.0;
    }
    // 4p team bonus handled in apply_team
}

/// Team bonus: 4p gives all allies Break Effect +30% for 2 turns after wearer uses Ult
/// on an ally — timing-conditional, not applied statically.
pub fn apply_team(_team: &mut Vec<TeamMember>, _relic_lists: &[Vec<IncomingRelic>]) {}
