//! Revelry by the Sea
//!
//! (2p) ATK +12%.
//!      ATK >= 2400 → DoT DMG +12%.
//!      ATK >= 3600 → DoT DMG +24% (replaces the 12% tier).
//!      ATK 2400 is achievable for most DoT DPS → +12% modelled as general DMG boost.

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.atk_percent += 12.0;
    let base_atk = member.base_stats.get(ids::CHAR_ATK_ID).copied().unwrap_or(0.0);
    let atk = base_atk * (1.0 + member.buffs.atk_percent / 100.0);
    if atk >= 3600.0 {
        member.buffs.dmg_boost += 24.0;
    } else if atk >= 2400.0 {
        member.buffs.dmg_boost += 12.0;
    }
}
