//! Belobog of the Architects
//!
//! (2p) DEF +15%.
//!      When Effect Hit Rate >= 50%, gain an extra DEF +15% (total +30%).

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.def_percent += 15.0;
    if member.buffs.effect_hit_rate >= 50.0 {
        member.buffs.def_percent += 15.0;
    }
}
