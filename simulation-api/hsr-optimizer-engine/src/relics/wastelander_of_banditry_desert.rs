//! Wastelander of Banditry Desert
//!
//! (2p) Imaginary DMG +10%.
//! (4p) CRIT Rate +10% vs debuffed enemies; CRIT DMG +20% vs Imprisoned enemies.
//!      Both are conditional on enemy debuff/Imprisoned state — not applied statically.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Imaginary" {
        member.buffs.dmg_boost += 10.0;
    }
    // 4p: all bonuses conditional on enemy debuff state — not applied statically.
}
