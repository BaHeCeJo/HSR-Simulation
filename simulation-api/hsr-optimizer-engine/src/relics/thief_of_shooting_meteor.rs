//! Thief of Shooting Meteor
//!
//! (2p) Break Effect +16%.
//! (4p) Break Effect +16% more (total +32% for full set).
//!      When inflicting Weakness Break, regenerates 3 Energy (not modelled).

use crate::ids;
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        *member.base_stats.entry(ids::CHAR_BE_ID.to_string()).or_insert(0.0) += 16.0;
    }
    if count >= 4 {
        *member.base_stats.entry(ids::CHAR_BE_ID.to_string()).or_insert(0.0) += 16.0;
    }
}
