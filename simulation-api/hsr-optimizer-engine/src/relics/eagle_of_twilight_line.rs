//! Eagle of Twilight Line
//!
//! (2p) Wind DMG +10%.
//! (4p) After Ultimate, action is Advanced Forward by 25%.
//!      Action advance changes AV queue position and requires a simulation hook
//!      to model accurately. The static optimizer does not simulate AV advancement,
//!      so the 4p effect is not modelled here.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Wind" {
        member.buffs.dmg_boost += 10.0;
    }
    // 4p: action advance — requires AV-timeline hook, not modelled statically.
    let _ = count;
}
