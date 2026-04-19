//! Forge of the Kalpagni Lantern
//!
//! (2p) SPD +6% (guaranteed).
//!      Break Effect +40% when hitting Fire-weak enemies — enemy weakness unknown.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.speed_percent += 6.0;
    // BE +40% requires Fire-weak enemy — not applied statically.
}
