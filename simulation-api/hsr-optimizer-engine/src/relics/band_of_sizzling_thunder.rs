//! Band of Sizzling Thunder
//!
//! (2p) Lightning DMG +10%.
//! (4p) When the wearer uses Skill, ATK +20% for 1 turn.
//!      Tracked via `band_skill_window` stack; applied in `apply_action_conditional_buffs`.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Lightning" {
        member.buffs.dmg_boost += 10.0;
    }
    // 4p: Skill-triggered ATK buff — handled dynamically in relics::on_action_used / apply_action_conditional_buffs.
}
