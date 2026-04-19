//! Rutilant Arena
//!
//! (2p) CRIT Rate +8%.
//!      When CRIT Rate >= 70%, Basic ATK and Skill DMG +20%.
//!      Conditional bonus applied only if the wearer's CRIT Rate actually reaches 70%.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.crit_rate += 8.0;
    // Check the real CRIT Rate (including the just-applied 8%) against the threshold.
    if member.buffs.crit_rate >= 70.0 {
        member.buffs.basic_atk_dmg_boost += 20.0;
        member.buffs.skill_dmg_boost     += 20.0;
    }
}
