//! Guard of Wuthering Snow
//!
//! (2p) DMG taken -8%.
//! (4p) At turn start with HP ≤ 50%: restore 8% Max HP and regenerate 5 Energy.
//!      The 4p effect fires in `relics::apply_turn_start_effects` each turn.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.incoming_dmg_reduction += 8.0;
    }
    // 4p: handled at turn start in relics::apply_turn_start_effects.
}
