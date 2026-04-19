//! Poet of Mourning Collapse
//!
//! (2p) Quantum DMG +10%.
//! (4p) SPD -8%.  CRIT Rate +20% if SPD < 110; CRIT Rate +32% if SPD < 95.
//!      This set intentionally makes characters slow to exchange SPD for CRIT Rate.

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Quantum" {
        member.buffs.dmg_boost += 10.0;
    }
    if count >= 4 {
        member.buffs.speed_percent -= 8.0;
        let base_spd = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        let spd = base_spd * (1.0 + member.buffs.speed_percent / 100.0);
        if spd < 95.0 {
            member.buffs.crit_rate += 32.0;
        } else if spd < 110.0 {
            member.buffs.crit_rate += 20.0;
        }
    }
}
