//! Duran, Dynasty of Running Wolves
//!
//! (2p) When allies use follow-up attacks, the wearer receives 1 "Merit" stack
//!      (max 5). Each stack: follow-up DMG +5%. At 5 stacks: CRIT DMG +25%.
//!      Stacks and CRIT DMG bonus only apply for FUA characters (is_fua = true).

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    if member.is_fua {
        member.buffs.follow_up_dmg_boost += 25.0; // max stacks for FUA chars
        member.buffs.crit_dmg            += 25.0; // 5-stack bonus
    }
}
