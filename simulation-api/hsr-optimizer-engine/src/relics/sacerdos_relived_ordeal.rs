//! Sacerdos' Relived Ordeal
//!
//! (2p) SPD +6%.
//! (4p) When using Skill or Ultimate on one ally target, that ally's CRIT DMG +18%
//!      for 2 turns (max 2 stacks = +36%).
//!      Applied as a team bonus in `apply_team` (called from mod.rs).
//!      Approximated as 1 stack always active on all allies = +18% CRIT DMG.

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "sacerdos_relived_ordeal";

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.speed_percent += 6.0;
    }
    // 4p wearer benefit: none directly (CRIT DMG goes to the ally target, not the wearer).
}

/// Team bonus: 4p gives CRIT DMG +18% to the ally targeted by wearer's Skill/Ult —
/// timing-conditional and single-target, not applied statically.
pub fn apply_team(_team: &mut Vec<TeamMember>, _relic_lists: &[Vec<IncomingRelic>]) {}
