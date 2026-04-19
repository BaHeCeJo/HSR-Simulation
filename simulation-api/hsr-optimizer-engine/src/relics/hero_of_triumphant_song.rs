//! Hero of Triumphant Song
//!
//! (2p) ATK +12%.
//! (4p) While memosprite is on field: SPD +6%.
//!      When memosprite attacks: wearer + memo CRIT DMG +30% for 2 turns.
//!      4p only applies to characters with an active memosprite (has_memo = true).

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.atk_percent += 12.0;
    }
    if count >= 4 && member.has_memo {
        member.buffs.speed_percent += 6.0;
        member.buffs.crit_dmg      += 30.0;
    }
}
