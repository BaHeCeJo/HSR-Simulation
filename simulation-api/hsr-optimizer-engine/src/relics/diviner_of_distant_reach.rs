//! Diviner of Distant Reach
//!
//! (2p) SPD +6%.
//! (4p) CRIT Rate +10% if SPD ≥ 120; CRIT Rate +18% if SPD ≥ 160.

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.speed_percent += 6.0;
    }
    if count >= 4 {
        let base_spd = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        let spd = base_spd * (1.0 + member.buffs.speed_percent / 100.0);
        if spd >= 160.0 {
            member.buffs.crit_rate += 18.0;
        } else if spd >= 120.0 {
            member.buffs.crit_rate += 10.0;
        }
    }
}
