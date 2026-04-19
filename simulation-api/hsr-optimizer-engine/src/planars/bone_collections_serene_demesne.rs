//! Bone Collection's Serene Demesne
//!
//! (2p) Max HP +12%.
//!      When Max HP >= 5000, wearer and memosprite gain CRIT DMG +28%.
//!      HP threshold easily met for max-level characters → both bonuses applied.

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.hp_percent += 12.0;
    // Compute approximate HP to check threshold.
    let base_hp = member.base_stats.get(ids::CHAR_HP_ID).copied().unwrap_or(0.0);
    let hp = base_hp * (1.0 + member.buffs.hp_percent / 100.0);
    if hp >= 5000.0 {
        member.buffs.crit_dmg += 28.0;
    }
}
