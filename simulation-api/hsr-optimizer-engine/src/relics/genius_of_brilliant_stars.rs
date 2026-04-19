//! Genius of Brilliant Stars
//!
//! (2p) Quantum DMG +10%.
//! (4p) DEF ignore 10% (guaranteed base).
//!      Additional 10% if target has Quantum Weakness — enemy weakness state unknown.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Quantum" {
        member.buffs.dmg_boost += 10.0;
    }
    if count >= 4 {
        member.buffs.def_ignore += 10.0; // base guaranteed; Quantum Weakness bonus not applied
    }
}
