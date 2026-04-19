//! Ever-Glorious Magical Girl
//!
//! (2p) CRIT DMG +16%.
//! (4p) Elation DMG ignores 10% of target's DEF (guaranteed base).
//!      Additional DEF ignore scales with Punchline stacks — not assumed at setup.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.crit_dmg += 16.0;
    }
    if count >= 4 {
        member.buffs.def_ignore += 10.0; // base guaranteed; stack bonus not applied
    }
}
