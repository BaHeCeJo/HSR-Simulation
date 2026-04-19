//! Firesmith of Lava-Forging
//!
//! (2p) Fire DMG +10%.
//! (4p) Skill DMG +12% (guaranteed).
//!      After Ultimate, Fire DMG +12% for the next attack — conditional on ult
//!      timing, not assumed at setup time.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Fire" {
        member.buffs.dmg_boost += 10.0;
    }
    if count >= 4 {
        member.buffs.skill_dmg_boost += 12.0;
        // Post-ult Fire DMG bonus: triggered per-attack after Ult — not applied statically.
    }
}
