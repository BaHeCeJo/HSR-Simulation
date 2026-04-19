//! Iron Cavalry Against the Scourge
//!
//! (2p) Break Effect +16%.
//! (4p) If Break Effect ≥ 150%: ignore 10% DEF for Break DMG.
//!      If Break Effect ≥ 250%: additionally ignore 15% DEF for Super Break DMG.
//!      Modelled as general DEF ignore (cannot distinguish Break vs normal DMG statically).

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        *member.base_stats.entry(ids::CHAR_BE_ID.to_string()).or_insert(0.0) += 16.0;
    }
    if count >= 4 {
        let be = member.base_stats.get(ids::CHAR_BE_ID).copied().unwrap_or(0.0);
        if be >= 250.0 {
            member.buffs.def_ignore += 25.0; // 10% + 15%
        } else if be >= 150.0 {
            member.buffs.def_ignore += 10.0;
        }
    }
}
