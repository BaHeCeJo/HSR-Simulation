//! Longevous Disciple
//!
//! (2p) Max HP +12%.
//! (4p) When hit or HP consumed by ally/self: CRIT Rate +8% for 2 turns (max 2 stacks).
//!      Tracked via `longevous_stacks` (0-2) + `longevous_window` (0-2 turns).
//!      Stack added in `relics::on_hit_taken`; read in `apply_action_conditional_buffs`;
//!      decremented each turn in `apply_turn_start_effects`.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.hp_percent += 12.0;
    }
    // 4p: CRIT Rate stacking — handled dynamically via on_hit_taken hook.
}
