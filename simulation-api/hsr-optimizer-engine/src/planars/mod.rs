//! Planar ornament set bonus dispatch.
//!
//! Each sub-module handles one ornament set (Sphere + Link Rope = 2 pieces).
//!
//! Adding a new ornament set:
//!   1. Create `src/planars/<name>.rs` with `pub fn apply(member)` and optionally
//!      `pub fn apply_team(team, relic_lists)`.
//!   2. Add `mod <name>;` below.
//!   3. Add the set_id string to the `has_set` call in `apply_set_bonuses`.
//!   4. If it has a team bonus, call it in `apply_team_set_bonuses`.

mod amphoreus_the_eternal_land;
mod arcadia_of_woven_dreams;
mod belobog_of_the_architects;
mod bone_collections_serene_demesne;
mod broken_keel;
mod celestial_differentiator;
mod city_of_converging_stars;
mod duran_dynasty_of_running_wolves;
mod firmament_frontline_glamoth;
mod fleet;
mod forge_of_the_kalpagni_lantern;
mod giant_tree_of_rapt_brooding;
mod inert_salsotto;
mod izumo_gensei_and_takama_divine_realm;
mod lushaka_the_sunken_seas;
mod pan_cosmic_commercial_enterprise;
mod penacony_land_of_the_dreams;
mod punklorde_stage_zero;
mod revelry_by_the_sea;
mod rutilant_arena;
mod sigonia_the_unclaimed_desolation;
mod space_sealing;
mod sprightly_vonwacq;
mod talia_kingdom_of_banditry;
mod tengoku_livestream;
mod the_wondrous_banamusement_park;

use crate::models::{IncomingRelic, TeamMember};

#[inline]
fn has_set(relics: &[IncomingRelic], set_id: &str) -> bool {
    relics.iter().any(|r| r.set_id == set_id)
}

/// Apply per-character planar ornament bonuses.
///
/// Call this AFTER relic main stats and relic set bonuses have been applied so
/// that SPD/EHR values are already in place for threshold checks.
pub fn apply_set_bonuses(member: &mut TeamMember, relics: &[IncomingRelic]) {
    if has_set(relics, "amphoreus_the_eternal_land")         { amphoreus_the_eternal_land::apply(member); }
    if has_set(relics, "arcadia_of_woven_dreams")            { arcadia_of_woven_dreams::apply(member); }
    if has_set(relics, "belobog_of_the_architects")          { belobog_of_the_architects::apply(member); }
    if has_set(relics, "bone_collections_serene_demesne")    { bone_collections_serene_demesne::apply(member); }
    if has_set(relics, "broken_keel")                        { broken_keel::apply(member); }
    if has_set(relics, "celestial_differentiator")           { celestial_differentiator::apply(member); }
    if has_set(relics, "city_of_converging_stars")           { city_of_converging_stars::apply(member); }
    if has_set(relics, "duran_dynasty_of_running_wolves")    { duran_dynasty_of_running_wolves::apply(member); }
    if has_set(relics, "firmament_frontline_glamoth")        { firmament_frontline_glamoth::apply(member); }
    if has_set(relics, "fleet_of_the_ageless")               { fleet::apply_per_wearer(member); }
    if has_set(relics, "forge_of_the_kalpagni_lantern")      { forge_of_the_kalpagni_lantern::apply(member); }
    if has_set(relics, "giant_tree_of_rapt_brooding")        { giant_tree_of_rapt_brooding::apply(member); }
    if has_set(relics, "inert_salsotto")                     { inert_salsotto::apply(member); }
    if has_set(relics, "izumo_gensei_and_takama_divine_realm") { izumo_gensei_and_takama_divine_realm::apply(member); }
    if has_set(relics, "lushaka_the_sunken_seas")            { lushaka_the_sunken_seas::apply(member); }
    if has_set(relics, "pan_cosmic_commercial_enterprise")   { pan_cosmic_commercial_enterprise::apply(member); }
    if has_set(relics, "penacony_land_of_the_dreams")        { penacony_land_of_the_dreams::apply(member); }
    if has_set(relics, "punklorde_stage_zero")               { punklorde_stage_zero::apply(member); }
    if has_set(relics, "revelry_by_the_sea")                 { revelry_by_the_sea::apply(member); }
    if has_set(relics, "rutilant_arena")                     { rutilant_arena::apply(member); }
    if has_set(relics, "sigonia_the_unclaimed_desolation")   { sigonia_the_unclaimed_desolation::apply(member); }
    if has_set(relics, "space_sealing_station")              { space_sealing::apply(member); }
    if has_set(relics, "sprightly_vonwacq")                  { sprightly_vonwacq::apply(member); }
    if has_set(relics, "talia_kingdom_of_banditry")          { talia_kingdom_of_banditry::apply(member); }
    if has_set(relics, "tengoku_livestream")                 { tengoku_livestream::apply(member); }
    if has_set(relics, "the_wondrous_banamusement_park")     { the_wondrous_banamusement_park::apply(member); }
}

/// Apply team-wide ornament bonuses that depend on the full team composition.
///
/// Call this AFTER `apply_set_bonuses` for every character.
/// `relic_lists[i]` must correspond to `team[i]`.
pub fn apply_team_set_bonuses(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    amphoreus_the_eternal_land::apply_team(team, relic_lists);
    broken_keel::apply_team(team, relic_lists);
    city_of_converging_stars::apply_team(team, relic_lists);
    fleet::apply_team(team, relic_lists);
    izumo_gensei_and_takama_divine_realm::apply_team(team, relic_lists);
    lushaka_the_sunken_seas::apply_team(team, relic_lists);
    penacony_land_of_the_dreams::apply_team(team, relic_lists);
}

/// Re-evaluate Effect RES-gated bonuses after battle-start hooks have applied
/// character minor traces and character-specific Effect RES grants (e.g. Aventurine talent).
/// Safe to call a second time — internal guards prevent double-application.
pub fn apply_effect_res_bonuses(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    broken_keel::apply_team(team, relic_lists);
}
