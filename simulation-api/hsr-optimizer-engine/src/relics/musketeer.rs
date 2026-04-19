//! Musketeer of Wild Wheat
//!
//! (2p) ATK +12%
//! (4p) SPD +6% | Basic ATK DMG +10%

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.atk_percent += 12.0;
    }
    if count >= 4 {
        member.buffs.speed_percent       += 6.0;
        member.buffs.basic_atk_dmg_boost += 10.0;
    }
}
