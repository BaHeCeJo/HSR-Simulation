//! Broken Keel
//!
//! (2p) Effect RES +8%.
//! (4p-equivalent team bonus) If any wearer's Effect RES ≥ 30%: all allies CRIT DMG +10%.

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "broken_keel";

/// Per-wearer bonus: Effect RES +8%.
pub fn apply(member: &mut TeamMember) {
    member.buffs.effect_res += 8.0;
}

/// Team bonus: any Broken Keel wearer with Effect RES ≥ 30% grants all allies CRIT DMG +10%.
///
/// Safe to call multiple times — guarded by `broken_keel_cdmg_applied` stack flag
/// so the +10% is only granted once even if this runs both at setup time and
/// after battle-start minor-trace application.
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_qualifying = relic_lists.iter().enumerate().any(|(i, r)| {
        let has_keel = r.iter().any(|p| p.set_id == SET_ID);
        if !has_keel { return false; }
        team[i].buffs.effect_res >= 30.0
    });
    if any_qualifying {
        for member in team.iter_mut() {
            if member.stacks.get("broken_keel_cdmg_applied").copied().unwrap_or(0.0) < 1.0 {
                member.buffs.crit_dmg += 10.0;
                member.stacks.insert("broken_keel_cdmg_applied", 1.0);
            }
        }
    }
}
