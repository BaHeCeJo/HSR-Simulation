//! Scholar Lost in Erudition
//!
//! (2p) CRIT Rate +8%.
//! (4p) Skill DMG +20%, Ultimate DMG +20% (both guaranteed).
//!      Post-ult Skill bonus +25% — conditional on ult timing, not applied statically.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.crit_rate += 8.0;
    }
    if count >= 4 {
        member.buffs.skill_dmg_boost += 20.0;
        member.buffs.ult_dmg_boost   += 20.0;
        // Post-ult extra +25% Skill DMG: triggered after each Ult — not applied statically.
    }
}
