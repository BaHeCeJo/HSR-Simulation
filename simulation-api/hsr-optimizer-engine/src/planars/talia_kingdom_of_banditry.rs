//! Talia: Kingdom of Banditry
//!
//! (2p) Break Effect +16%.
//!      When SPD >= 145, Break Effect +20% more (total +36%).

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    *member.base_stats.entry(ids::CHAR_BE_ID.to_string()).or_insert(0.0) += 16.0;
    let base = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    let spd  = base * (1.0 + member.buffs.speed_percent / 100.0);
    if spd >= 145.0 {
        *member.base_stats.entry(ids::CHAR_BE_ID.to_string()).or_insert(0.0) += 20.0;
    }
}
