//! Arcadia of Woven Dreams
//!
//! (2p) When there are more or less than 4 ally targets in battle, each
//!      additional/missing ally increases the wearer's DMG by 9%/12% (max 4/3 stacks).
//!      In a standard 4-ally fight, the bonus is 0. No-op for full-team scenarios.

use crate::models::TeamMember;

pub fn apply(_member: &mut TeamMember) {
    // Standard 4-ally team → 0 stacks → no bonus.
    // If the battle has fewer allies (e.g. 3), each missing ally = +12% DMG (up to 3 stacks).
    // The optimizer always simulates full teams, so this is a no-op here.
}
