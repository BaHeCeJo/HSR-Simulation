//! Hunter of Glacial Forest
//!
//! (2p) Ice DMG +10%.
//! (4p) After Ultimate, CRIT DMG +25% for 2 turns.
//!      Conditional on ult being used — not applied statically.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Ice" {
        member.buffs.dmg_boost += 10.0;
    }
    // 4p: post-ult CRIT DMG — triggered per-turn after Ult, not applied statically.
}
