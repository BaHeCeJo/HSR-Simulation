//! Tengoku@Livestream
//!
//! (2p) CRIT DMG +16%.
//!      If 3+ Skill Points are consumed in the same turn, CRIT DMG +32% for 3 turns.
//!      Approximated as CRIT DMG +16% additional (50% uptime for skill-heavy rotations).
//!      Total: CRIT DMG +32%.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.crit_dmg += 16.0;       // base
    member.buffs.crit_dmg += 16.0;       // 50% uptime of +32% conditional
}
