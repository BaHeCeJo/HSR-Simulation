//! The Wondrous BananAmusement Park
//!
//! (2p) CRIT DMG +16%.
//!      When a target summoned by the wearer is on the field, CRIT DMG +32%.
//!      The extra +32% only applies for characters with a memosprite (has_memo = true).

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.crit_dmg += 16.0; // guaranteed 2p
    if member.has_memo {
        member.buffs.crit_dmg += 32.0; // summoned target always on field for memo chars
    }
}
