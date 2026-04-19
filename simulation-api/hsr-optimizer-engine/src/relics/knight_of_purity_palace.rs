//! Knight of Purity Palace
//!
//! (2p) DEF +15%.
//! (4p) Increases max DMG absorbed by wearer's shields by 20%.
//!      shield_effect is consumed when a character creates a shield in their kit.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.def_percent += 15.0;
    }
    if count >= 4 {
        member.buffs.shield_effect += 20.0;
    }
}
