//! Space Sealing Station
//!
//! (2p) ATK +12%.
//!      If wearer SPD >= 120, ATK +12% more (total +24%).

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.atk_percent += 12.0;
    let base = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    let spd  = base * (1.0 + member.buffs.speed_percent / 100.0);
    if spd >= 120.0 {
        member.buffs.atk_percent += 12.0;
    }
}
