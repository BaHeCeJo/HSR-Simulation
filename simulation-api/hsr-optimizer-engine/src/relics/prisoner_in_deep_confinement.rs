//! Prisoner in Deep Confinement
//!
//! (2p) ATK +12%.
//! (4p) For every DoT on the target, ignore 6% DEF (max 3 DoTs = 18% DEF ignore).
//!      Applied dynamically in `relics::apply_action_conditional_buffs` by counting
//!      actual DoTs (Burn, Bleed, Shock, Wind Shear, Arcana, Entanglement) on the target.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.atk_percent += 12.0;
    }
    // 4p: DEF ignore — handled dynamically in relics::apply_action_conditional_buffs.
}
