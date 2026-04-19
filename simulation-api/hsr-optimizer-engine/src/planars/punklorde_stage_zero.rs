//! Punklorde Stage Zero
//!
//! (2p) Elation (Remembrance path charge) +8%.
//!      At 40% Elation: CRIT DMG +20%. At 80%: CRIT DMG +32%.
//!      Elation is a Remembrance-exclusive mechanic — bonuses only apply to
//!      Remembrance-path characters.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    // Elation charge only exists for Remembrance-path characters.
    if member.path == "Remembrance" {
        member.buffs.crit_dmg += 20.0; // 40% threshold reachable in rotation
    }
}
