//! Messenger Traversing Hackerspace
//!
//! (2p) SPD +6%.
//! (4p) When wearer uses Ultimate on an ally, all allies SPD +12% for 1 turn.
//!      Applied as a team bonus in `apply_team` (called from mod.rs).
//!      Approximated at ~50% uptime → +6% effective team SPD.

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "messenger_traversing_hackerspace";

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.speed_percent += 6.0;
    }
    // 4p wearer benefit: none (all allies SPD buff — handled in apply_team).
}

/// Team bonus: 4p triggers SPD +12% for all allies for 1 turn after wearer uses Ult
/// on an ally — timing-conditional, not applied statically.
pub fn apply_team(_team: &mut Vec<TeamMember>, _relic_lists: &[Vec<IncomingRelic>]) {}
