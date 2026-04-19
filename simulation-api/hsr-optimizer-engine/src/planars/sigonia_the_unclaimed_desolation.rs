//! Sigonia, the Unclaimed Desolation
//!
//! (2p) CRIT Rate +4% (guaranteed).
//!      When an enemy is defeated, CRIT DMG +4% per stack (max 10 stacks).
//!      Kill-triggered stacking — stack count depends on combat history, not applied statically.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.crit_rate += 4.0;
    // Kill-triggered CRIT DMG stacking — not applied statically.
}
