//! Pioneer Diver of Dead Waters
//!
//! (2p) All DMG +12% when hitting debuffed enemies.
//! (4p) CRIT Rate +4%, CRIT DMG +24% when target has ≥2 debuffs.
//!      Both bonuses are conditional on enemy debuff state — not applied statically.

use crate::models::TeamMember;

pub fn apply(_member: &mut TeamMember, _count: usize) {
    // All bonuses require enemy debuff state which is unknown at setup time.
}
