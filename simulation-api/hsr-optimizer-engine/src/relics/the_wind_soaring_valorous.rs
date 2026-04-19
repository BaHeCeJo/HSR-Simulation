//! The Wind-Soaring Valorous
//!
//! (2p) ATK +12%.
//! (4p) CRIT Rate +6% (always active).
//!      After using a follow-up attack, Ultimate DMG +36% for 1 turn.
//!      The post-FUA Ult DMG window is set by `relics::on_follow_up_end` and tracked via
//!      `wind_soaring_fua_window`; read in `apply_action_conditional_buffs` for Ult actions;
//!      decremented in `apply_turn_start_effects`.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.atk_percent += 12.0;
    }
    if count >= 4 {
        member.buffs.crit_rate += 6.0; // always active (unconditional)
        // 4p Ult DMG +36% after FUA — handled dynamically via on_follow_up_end hook.
    }
}
