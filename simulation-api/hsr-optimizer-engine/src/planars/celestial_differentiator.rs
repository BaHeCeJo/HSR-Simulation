//! Celestial Differentiator
//!
//! (2p) CRIT DMG +16%.
//!      When current CRIT DMG >= 120%, CRIT Rate +60% until end of first attack.
//!      The CR boost lasts only one attack per battle — negligible average impact.
//!      Approximated as CRIT DMG +16% only.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.crit_dmg += 16.0;
    // The conditional CR+60% applies for 1 attack; not modelled in static optimizer.
}
