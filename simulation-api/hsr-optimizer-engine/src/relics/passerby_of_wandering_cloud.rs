//! Passerby of Wandering Cloud
//!
//! (2p) Outgoing Healing +10%.
//! (4p) At battle start, immediately recover 1 Skill Point.
//!      The 4p effect fires in `relics::apply_battle_start_effects`.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.outgoing_healing += 10.0;
    }
    // 4p: +1 SP at battle start — handled in relics::apply_battle_start_effects.
}
