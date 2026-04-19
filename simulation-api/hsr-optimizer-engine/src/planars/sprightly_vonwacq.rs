//! Sprightly Vonwacq
//!
//! (2p) Energy Regeneration Rate +5%.
//!      When SPD >= 120, action Advanced Forward by 40% upon entering battle.
//!      Action advance cannot be modelled in the static optimizer.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.energy_regen_rate += 5.0;
    // Action advance (SPD >= 120) not modelled in static optimization.
}
