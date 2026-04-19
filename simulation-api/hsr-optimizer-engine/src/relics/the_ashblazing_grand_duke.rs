//! The Ashblazing Grand Duke
//!
//! (2p) Follow-up attack DMG +20%.
//! (4p) When the wearer uses follow-up attacks, ATK +6% per hit (max 8 stacks, 3 turns).
//!      Stack resets at the start of each new follow-up sequence.
//!      Tracked via `ashblazing_stacks` (0-8) + `ashblazing_window` (0-3 turns).
//!      Updated by `relics::on_follow_up_start/hit`; read in `apply_action_conditional_buffs`;
//!      decremented in `apply_turn_start_effects`.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.follow_up_dmg_boost += 20.0;
    }
    // 4p: ATK stacking — handled dynamically via on_follow_up_start/hit hooks.
}
