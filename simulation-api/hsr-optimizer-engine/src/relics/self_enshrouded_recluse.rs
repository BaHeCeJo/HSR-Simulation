//! Self-Enshrouded Recluse
//!
//! (2p) Shield Effect +10%.
//! (4p) Shield Effect provided by wearer +12%.
//!      When an ally has a Shield from the wearer, that ally's CRIT DMG +15%.
//!      The CRIT DMG buff requires a live shield — not applied statically.

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "self_enshrouded_recluse";

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.shield_effect += 10.0;
    }
    if count >= 4 {
        member.buffs.shield_effect += 12.0;
        // 4p: CRIT DMG +15% to shielded ally — requires active shield, not applied statically.
    }
}

/// Team CRIT DMG bonus: flag all members so apply_action_conditional_buffs can grant +15%
/// CRIT DMG dynamically whenever a member has an active shield.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_4p = relic_lists.iter()
        .any(|r| r.iter().filter(|p| p.set_id == SET_ID).count() >= 4);
    if any_4p {
        for member in team.iter_mut() {
            member.stacks.insert("recluse_crit_available", 1.0);
        }
    }
}
