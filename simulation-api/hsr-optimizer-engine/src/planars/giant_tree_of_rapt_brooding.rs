//! Giant Tree of Rapt Brooding
//!
//! (2p) SPD +6%.
//!      SPD >= 135 → Outgoing Healing +12%.
//!      SPD >= 180 → Outgoing Healing +20% (replaces the 12% tier).

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.speed_percent += 6.0;
    let base = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    let spd  = base * (1.0 + member.buffs.speed_percent / 100.0);
    if spd >= 180.0 {
        member.buffs.outgoing_healing += 20.0;
    } else if spd >= 135.0 {
        member.buffs.outgoing_healing += 12.0;
    }
}
