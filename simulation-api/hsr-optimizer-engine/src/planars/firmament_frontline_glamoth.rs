//! Firmament Frontline: Glamoth
//!
//! (2p) ATK +12%.
//!      SPD >= 135 → DMG +12%.
//!      SPD >= 160 → DMG +18% (replaces the 12% tier).

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.atk_percent += 12.0;
    let base = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    let spd  = base * (1.0 + member.buffs.speed_percent / 100.0);
    if spd >= 160.0 {
        member.buffs.dmg_boost += 18.0;
    } else if spd >= 135.0 {
        member.buffs.dmg_boost += 12.0;
    }
}
