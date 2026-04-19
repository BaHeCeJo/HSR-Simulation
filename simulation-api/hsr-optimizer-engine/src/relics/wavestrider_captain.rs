//! Wavestrider Captain
//!
//! (2p) CRIT DMG +16%.
//! (4p) "Help" stacks from ally targeting (max 2); at 2 stacks on Ult use: ATK +48%.
//!      Stack count and ult timing not predictable at setup time — not applied statically.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.crit_dmg += 16.0;
    }
    // 4p: stack-based ATK burst — not applied statically.
}
