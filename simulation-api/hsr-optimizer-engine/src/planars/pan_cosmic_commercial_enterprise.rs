//! Pan-Cosmic Commercial Enterprise
//!
//! (2p) Effect Hit Rate +10%.
//!      ATK += 25% of current Effect Hit Rate, up to +25% ATK.

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember) {
    member.buffs.effect_hit_rate += 10.0;
    let ehr_bonus = (member.buffs.effect_hit_rate * 0.25_f64).min(25.0);
    member.buffs.atk_percent += ehr_bonus;
}
