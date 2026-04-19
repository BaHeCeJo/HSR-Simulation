//! Champion of Streetwise Boxing
//!
//! (2p) Physical DMG +10%.
//! (4p) After attacking or being hit, ATK +5% per stack (up to 5×).
//!      Stack count depends on combat history — not assumed at setup time.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Physical" {
        member.buffs.dmg_boost += 10.0;
    }
    // 4p: stacking ATK bonus — not applied statically (stack count unknown).
}
