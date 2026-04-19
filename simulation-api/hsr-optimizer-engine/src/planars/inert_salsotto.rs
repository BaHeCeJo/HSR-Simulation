//! Inert Salsotto
//!
//! (2p) CRIT Rate +8%.
//!      When current CRIT Rate >= 50%, Ultimate and follow-up attack DMG +15%.
//!      Conditional bonus applied only if the wearer's CRIT Rate actually reaches 50%.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.crit_rate += 8.0;
    // Check the real CRIT Rate (including the just-applied 8%) against the threshold.
    if member.buffs.crit_rate >= 50.0 {
        member.buffs.ult_dmg_boost       += 15.0;
        member.buffs.follow_up_dmg_boost += 15.0;
    }
}
