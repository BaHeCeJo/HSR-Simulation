# Simulation Approximations & Assumptions

This file tracks every approximation, simplification, and skipped mechanic in the engine.
Each entry notes the file, what was approximated, and how to fix it if desired.

---

## Characters

### Aventurine (`characters/aventurine.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | Bouncing Ball extra stack gain | Hardcoded to **4** (middle of 1–7 range) | Roll 1d7 or use 4 as the median |

---

### Clara (`characters/clara.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | E6: 50% chance counter when *another ally* is attacked | Deterministic alternating flip (`flip < 0.5`) — hits every other proc | True 50% RNG, or keep alternating |
| 2 | `on_enemy_action` does not know which ally was targeted | Counter always fires (Clara has highest aggro, approximated as always targeted) | Pass target ally idx to hook |

---

### Dan Heng (`characters/dan_heng.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | A4: 50% chance +20% SPD for 2 turns after each attack | Deterministic alternating flip — triggers on every other attack | True 50% RNG |
| 2 | Talent RES PEN trigger: "ally targets Dan Heng with an Ability" | Approximated as any ally using a **Skill** action | Check if source ally actually targeted DH with their action |

---

### Dan Heng • Imbibitor Lunae (`characters/dan_heng_il.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | Righteous Heart (DMG% per hit) | Average stack count across hits applied as a single flat `dmg_boost` before damage calc | Simulate each hit individually with its real stack count |
| 2 | Outroar (CRIT DMG per blast-hit) | Average stack count across hits applied as a single flat `crit_dmg` before damage calc | Simulate each hit individually with its real stack count |
| 3 | Squama Sacrosancta usage | Tracked in stacks but not consumed for SP refund interactions beyond basic Dracore Libre | Full SP-based gate mechanic |

---

### Dan Heng • Permansor Terrae (`characters/dan_heng_pt.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | A4: 40% action advance at battle start | **NOT MODELED** — initial AV queue is set before `on_battle_start` runs | Modify `run_simulation` to re-push initial AV entries after `on_battle_start` |
| 2 | Bondmate selection | Heuristic: lowest-path-aggro ally (The Hunt / Erudition first) — not player-chosen | Accept explicit bondmate from request payload |
| 3 | A2 ATK boost to Bondmate | Applied once at battle start; not refreshed on each Skill use | Re-apply on each Skill, tracking and reverting previous amount |
| 4 | Non-enhanced Souldragon turns | Shield and debuff-cleanse effects are **skipped** (no DPS impact) | N/A for DPS sim; add healing/cleanse sim if needed |
| 5 | E6 DEF ignore (12%) on Bondmate | Applied permanently at designation; not removed if Bondmate changes | Track and revert if Bondmate is re-designated |

---

### Fu Xuan (`characters/fu_xuan.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | DMG distribution to Fu Xuan (65% of ally damage) | **NOT MODELED** — no ally-damage-received hook exists | Add `on_ally_hit_taken` hook; route 65% of ally HP loss to Fu Xuan when MoP is active |
| 2 | Knowledge HP boost (+6% Fu Xuan Max HP to each ally) | **SKIPPED** — flat HP boost based on another character's stat can't cleanly fit in `buffs.hp_percent` | Track a flat HP delta per ally; add/subtract on MoP apply/remove |
| 3 | MoP Knowledge timing for allies | Knowledge for all others is applied in `on_after_action` (snapshot-safe); Fu Xuan's own buff is applied in next `on_turn_start` — so she misses Knowledge for the turn the Skill is used | Apply Fu Xuan's own buff immediately but revert-safe (e.g. persist to a separate accumulator) |
| 4 | MoP expiry removes Knowledge at Fu Xuan's TURN START | Other allies technically lose Knowledge during Fu Xuan's turn start, not at the natural expiry (end of the 3rd ally turn cycle) | Expire via a global turn-tick rather than on Fu Xuan's turn start |
| 5 | A6: Crowd Control immunity during MoP | **NOT MODELED** — no DPS impact | Add a flag to `on_enemy_action` that blocks CC debuff application |
| 6 | E2: KO prevention for all allies (1× per battle) | **NOT MODELED** — defensive only | Check for killing-blow condition in `apply_damage_to_ally`; if MoP active and E2 unused, prevent KO |
| 7 | HP Restore fires less frequently than in-game | Because DMG distribution is not modeled, Fu Xuan's HP drops only from direct attacks, not 65% of ally damage re-routed to her | Implement DMG distribution (see #1) |
| 8 | E6 HP loss tally: only captures enemy-attack damage | Tally updated only in `on_enemy_action`; HP loss from other sources (DoT, execution, etc.) not tracked | Hook into `apply_damage_to_ally` to record every HP decrease |
| 9 | Technique pre-battle MoP (2 turns) | **NOT MODELED** — no technique-state flag exists | Set a flag in `on_battle_start` if technique was used; activate MoP for 2 turns |

---

### Firefly / SAM (`characters/firefly.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | CC countdown timer (SPD 70) | Duration = `1 + floor(CC_timer_AV × FF_SPD_in_CC / 10000)` turns; first turn is "free" from 100% advance | Push a real countdown AV entry into `av_queue` that fires `exit_cc` |
| 2 | A2: countdown delay on break | Each Weakness Break in CC adds **+0.3 CC turns** (approximates 10% of timer at ~194 SPD); max 3 breaks | Compute exact AV delay: `0.1 × 10000/70 / (10000/(FF_SPD+60))` |
| 3 | Enhanced Skill adjacent blast | Only main-target damage simulated; adjacent hit `(0.1×BE+100%)` ATK **skipped** | Track adjacent enemy slots; apply scaled hit to each |
| 4 | E2: extra turn on kill/break | **NOT MODELED** — kill/break detection mid-action is not wired into on_after_action | Detect kill in on_after_action; re-insert Firefly at current_av+ε |
| 5 | A4 super break source attribution | `on_break` credits A2 delay to Firefly whenever any enemy breaks during CC, regardless of who broke it | Pass attacker idx through `dispatch_on_break` |
| 6 | SPD +60 flat | Added/removed directly to `base_stats[CHAR_SPD_ID]` at CC entry/exit | Already correct; no approximation, just unusual storage location |
| 7 | Talent HP-based DMG reduction | Pre-CC reduction (up to 40% at ≤20% HP) **skipped**; during CC always 40% applied | Sample current HP% in on_turn_start and set `incoming_dmg_reduction` accordingly |
| 8 | Normal Skill action advance (+25%) | `_action_advance_pct = 25` set in on_after_action; engine applies as SPD/(1-0.25) for next reschedule | Already the correct mechanism |

---

### Gepard (`characters/gepard.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | Skill damage/freeze multipliers | Fixed max-level values used: Basic **130%** ATK, Skill **240%** ATK, frozen additional **80%** ATK; exact values require ability-level lookup | Read `ability_levels.basic / skill` and interpolate the scaling table |
| 2 | Freeze base chance | Accumulator: +**0.65** per Skill (1.0 with E1); triggers deterministically every ~1.5 Skills instead of true 65% RNG | True 65% (or 100% with E1) RNG per Skill cast |
| 3 | Freeze EHR/EffRES interaction | Base chance used as-is; Gepard's Effect Hit Rate and enemy Effect RES are **not applied** | Multiply by `(1 + EHR) × (1 − EffRES)` |
| 4 | Frozen enemy action prevention | Modeled as **additional Ice DMG only**; enemy still takes their turn (skip-turn mechanic cannot be applied from a character hook) | Intercept enemy AV entry in simulator and skip the action when `gepard_frozen_X > 0` |
| 5 | E2: post-freeze SPD reduction | **NOT MODELED** — AV queue manipulation from a hook is infeasible without simulator support | Modify the enemy's AV entry when freeze expires in `on_enemy_turn_start` |
| 6 | Ult shield duration (3 turns) | Shield is applied and **never expires** (persists until the next Ult re-application) — Aventurine uses the same simplification | Tick shield HP per shielded ally's turn via `on_ally_action` |
| 7 | Ice DMG minor trace | +22.4% Ice DMG applied as `dmg_boost` (all-DMG pool), not as a separate Ice-only multiplier | Add an `ice_dmg_boost` field to Buffs; multiply in `damage.rs` only when element is Ice |
| 8 | Talent (Unyielding Will) revival scheduling | Talent fires in `on_enemy_action` after HP ≤ 0; the simulator may have already removed Gepard from the AV queue, preventing him from taking future turns | Hook into the pre-death check before HP is zeroed, or re-insert Gepard's AV entry when talent fires |
| 9 | Technique shield (pre-battle) | **NOT MODELED** — no technique-state flag exists | Set a flag in `on_battle_start`; apply Ult shield formula for 2 turns |

---

### Gallagher (`characters/gallagher.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | Skill heal amount | Flat **1600 HP × (1 + Outgoing Healing%)** instead of game formula (9.6% target Max HP + 257) | Use target's Max HP in the heal formula |
| 2 | A2 Outgoing Healing computed once at battle start | `min(total_BE × 0.5, 75%)` is snapshotted; changes to BE mid-combat (e.g. relic 4p) do not update OH | Recompute in `on_turn_start` or whenever BE changes |
| 3 | Talent heal triggered by Gallagher's own attacks | Handled in `on_after_action` (Basic); `on_ally_action` handles all other allies (dispatcher skips self) | Unified hook that includes self |
| 4 | A6 heal requires Besotted pre-existing | A6 (+640 HP to allies) only fires when target was **already Besotted** before Nectar Blitz; first-ever Nectar Blitz after Ult doesn't A6-heal | Fire A6 unconditionally on any Nectar Blitz use if game description is unconditional |
| 5 | Besotted vulnerability applies to ALL damage types | `enemy.vulnerability += 12.0` boosts all damage (not just Break DMG) because `calculate_damage` uses `vulnerability + cached_vuln_bonus` | Separate a `break_vulnerability` field on `SimEnemy` for break-only effects |
| 6 | E4: Gallagher's Basic ATK ignores 40% DEF | **NOT MODELED** | Apply `buffs.def_ignore += 40.0` in `on_before_action` for Basic and revert in `on_after_action` |
| 7 | Technique buff (+30% BE for 3 turns) | **NOT MODELED** | Set a flag in `on_battle_start`; apply +30% BE for first 3 turns and expire |
| 8 | Besotted duration ticking | Ticked per **enemy turn** (`on_enemy_turn_start`); no auto-tick via `effects::tick_enemy_debuffs` since Besotted is not in `active_debuffs` | Migrate Besotted to `active_debuffs` once an appropriate vulnerability field exists |

---

### Guinaifen (`characters/guinaifen.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | Skill damage multipliers | Fixed max-level values: Basic **100%** ATK, Skill main **120%** + adjacent **40%**, Burn DoT **218.2%** ATK | Read `ability_levels` and interpolate the scaling table |
| 2 | Burn application chance | Skill applies Burn at **100% base chance** (always); game requires an Effect Hit Rate check for the initial application | Multiply by `(1 + EHR) × (1 − EffRES)` before applying |
| 3 | A2 Basic Burn chance via accumulator | +**0.80** per Basic attack; Burn fires deterministically every ~1.25 Basics instead of true 80% RNG | True 80% (or per-hit EHR-adjusted) RNG |
| 4 | Burn DoT no-crit approximation | `crit_rate` zeroed on the cloned member → `expected_crit = 1.0`; actual DoT never crits | Correct — DoTs do not crit in HSR; this is accurate |
| 5 | A6 +20% DMG boost applied to adjacent Skill hits | A6 reads live `is_burned(state, adj)` at hit time via cloned member; boost is correctly scoped to each target | No fix needed |
| 6 | Firekiss vulnerability on dead enemies | `apply_firekiss` can add stacks to enemies that die during the same Burn proc chain | Guard `state.enemies[slot].as_ref().map_or(false, |e| e.hp > 0.0)` is present in `burn_proc` — already handled |
| 7 | Firekiss +7% vulnerability applied via `enemy.vulnerability` | Same limitation as Besotted — all-damage vulnerability, not a separate fire-only or DoT-only multiplier | Add a `talent_vulnerability` field for Talent-specific boosts |
| 8 | A4: +25% action advance at battle start | **NOT MODELED** — AV queue is initialized before `on_battle_start` hooks fire | Initialize AV with A4 already applied, or advance Guinaifen's first AV entry post-init |

---

### Black Swan (`characters/black_swan.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | Skill damage multipliers | Fixed max-level values: Basic **100%** ATK, Skill main + adjacent **90%** ATK, Arcana DoT **240% + 12%/stack** ATK, adjacent DoT **180%** ATK | Read `ability_levels` and interpolate the scaling table |
| 2 | Talent Arcana gain per DoT | **1 DoT instance per Arcana tick** → 0.65 expected Arcana/turn; the virtual Wind Shear/Bleed/Burn/Shock from Arcana state are NOT counted as additional DoT instances for this trigger | Count virtual DoTs as real instances (×5 total) once game data confirms behavior |
| 3 | A2 / Talent / E6 Arcana chance via accumulator | 65% base chance modeled as +0.65 per trigger, fires at ≥ 1.0 — deterministic instead of RNG; A2 carries over fractional probability across different trigger sources in the same accumulator | Separate accumulators per source; true per-event RNG |
| 4 | Epiphany 50% chain → EV 2× | `effective *= 2.0` — treats the geometric chain as its closed-form expected value; discrete stack counts will differ from real gameplay | Simulate the chain: flip 50% coin repeatedly until fail |
| 5 | E1 All-RES reduction | `cached_all_res_reduce += 25.0` for ALL elements; game applies -25% **element-specific** (Wind for Wind Shear, Physical for Bleed, Fire for Burn, Lightning for Shock) | Add per-element RES reduction fields to `SimEnemy` |
| 6 | Epiphany duration in `active_buffs` | Vulnerability buff placed in `active_buffs` (not auto-ticked); duration manually decremented per `on_enemy_turn_start`; no automatic expiry if `on_enemy_turn_start` is not reached | Migrate to `active_debuffs` or add an `active_buffs` ticker in the simulator |
| 7 | A6 team DMG boost snapshotted at battle start | `60% × EHR` computed once and applied to all allies; later EHR changes (relic 4p sets, LC passives) are not reflected | Recompute in `on_turn_start` or whenever EHR changes |
| 8 | Arcana stack excess behavior | Stacks can exceed the cap during accumulation; cap applied after halving in `on_enemy_turn_start` — equivalent to the "excess removed after DMG" game rule | Already correct; no fix needed |
| 9 | Technique (pre-battle Arcana) | **NOT MODELED** — variable cascade with 150%/75%/37.5%... base chances | Apply cascade starting at 1.5× base chance in `on_battle_start` behind a technique flag |

---

### Bronya (`characters/bronya.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | E1: 50% chance to recover 1 SP after Skill | Accumulator — grants SP every **2** Skill uses (0.5 per use) | True 50% RNG |
| 2 | Bronya Skill advances ally action | Invalidates the stale AV entry for the target; assumes only one entry exists | Handle multiple entries for the same actor |

---

### Feixiao (`characters/feixiao.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | Ult activation timing | Ult can only fire at the **end of Feixiao's turn** (when `_ult_ready` is set in `on_turn_start`); in-game she can ult mid-cycle as soon as FA ≥ 6 | Check FA ≥ 6 after every FA gain and set `_ult_ready` immediately |
| 2 | Double-ult in one turn | If FA = 12, she fires 1 ult (FA → 6) and the 2nd ult happens next turn; in-game she could fire both this turn | After ult, re-check FA ≥ 6 and call execute_ult again if true |
| 3 | A6 +48% ATK on Skill | Applied from **next Feixiao turn** (on_turn_start); the Skill itself and the Skill-triggered FUA miss the A6 boost this turn | Apply in on_before_action (snapshot window) AND track persistently |
| 4 | Talent +60% DMG first FUA | The FUA that first ACTIVATES the buff gets +60% via clone only; Feixiao's Basic/Skill on that same turn don't get +60% (buff lands in buffs from next on_turn_start) | Immediately apply to buffs when FUA fires outside snapshot; defer for in-snapshot triggers |
| 5 | E4 +8% SPD | Applies from next Feixiao turn (same stacks-management as A6) | Same as A6 fix |
| 6 | Ult sub-hit type | Always 90% ATK (best of Boltsunder Blitz / Waraxe Skyward); break status mid-ult not re-evaluated | Track per-hit break status; switch hit type when enemy breaks during ult |
| 7 | Ult as FUA (A4) | `follow_up_dmg_boost` is added to `dmg_boost` in the member clone for ult hits; relic/LC effects that key on `ActionType::FollowUp` do NOT apply to ult hits | Use `ActionType::FollowUp` for ult sub-hits (would require ult_dmg_boost to be added separately) |
| 8 | Skill FUA vs Talent FUA | Skill's direct FUA clears `TALENT_USED=0` after firing so an ally can still trigger the Talent this turn; if that causes double-FA this cycle it's a minor gain | Track Skill FUA and Talent FUA separately |

---

### Dr. Ratio (`characters/dr_ratio.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | A4: 40% Talent FUA trigger per Skill | Threshold accumulator (fire when acc ≥ 1.0) — fires every 2–3 Skills instead of true 40% RNG | True 40% RNG per Skill |
| 2 | A6 +DMG boost on WF FUAs | A6 is NOT added inside `fire_talent_fua`; WF FUAs (called from `on_ally_action`) are outside the on_before_action snapshot window so the boost was already reverted | Apply A6 separately in `fire_talent_fua` based on debuff count |
| 3 | Summation stacks CR/CD timing | Stacks are stored in `team.stacks`, CR/CD re-applied in `on_turn_start` (before snapshot). On_after_action updates the raw stack count; the new CR/CD lands on the **next** turn | Re-apply immediately in on_after_action without snapshot interference |
| 4 | Wiseman's Folly on ult attack | `execute_ult` now calls `dispatch_on_ally_action` so WF triggers from Dr. Ratio's own ult hit | If the ult doesn't deal damage to the WF target, triggers would fire incorrectly |
| 5 | A4 Effect RES debuff duration | 2 turns — not refreshed by subsequent Skills | Refresh duration on each Skill application |

---

### Silver Wolf (`characters/silver_wolf.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | E2: triggers on any ally debuff infliction | `dispatch_on_ally_action` skips the acting character (self), so SW's own debuffs do **not** trigger E2 | Apply E2 in `on_after_action` when SW herself inflicts a debuff |

---

### Anaxa (`characters/anaxa.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | E6: +30% DMG bonus | Treated as **additive** to `dmg_boost` instead of a true 1.3× multiplicative multiplier | Multiply final damage by 1.3 instead of adding 30 to the boost pool |
| 2 | 2+ Erudition party buff (+50% DMG to all allies) | Simulated as **+50% vulnerability on enemies** instead of +50% DMG% on allies | Apply as `dmg_boost` to each ally rather than enemy vulnerability |

---

## Relics

### Champion of Streetwise Boxing (`relics/champion_of_streetwise_boxing.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: +5% ATK per stack (up to 5) accumulated during combat | Stack count at setup is **0** — only 2p Physical DMG +10% is applied | Simulate stack accumulation per attack during the fight |

### Firesmith of Lava-Forging (`relics/firesmith_of_lava_forging.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: +12% Fire DMG for the next attack after Ult | Window tracking **not modeled** | Track post-ult window flag; apply and consume on next hit |

### Eagle of Twilight Line (`relics/eagle_of_twilight_line.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: 25% action advance after Ult | **NOT MODELED** (AV queue manipulation from static relic hook is infeasible) | Set `_action_advance_pct` in a post-ult hook |

### Sacerdos' Relived Ordeal (`relics/sacerdos_relived_ordeal.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: +18% CRIT DMG per stack to targeted ally (max 2 stacks, 2 turns) | Approximated as **1 stack always active on all allies** (+18% CRIT DMG flat) | Track stacks per ally; apply only to the most-recently buffed ally |

### Watchmaker, Master of Dream Machinations (`relics/watchmaker_master_of_dream_machinations.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: +30% Break Effect to all allies for 2 turns after Ult on ally | Modeled as **50% uptime → +15% BE permanently** | Track Ult timing; apply and expire the buff properly |

### Messenger Traversing Hackerspace (`relics/messenger_traversing_hackerspace.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: +12% SPD to all allies for 1 turn after Ult on ally | Modeled as **50% uptime → +6% SPD permanently** | Track Ult timing; apply and expire per turn |

### Thief of Shooting Meteor (`relics/thief_of_shooting_meteor.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: regenerate 3 Energy on Weakness Break | **NOT MODELED** | Add energy grant in `dispatch_on_break` for the wearer |

### Ever-Glorious Magical Girl (`relics/ever_glorious_magical_girl.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: DEF ignore scales with Punchline stacks | Stack-scaled portion **skipped**; only base +10% DEF ignore applied | Simulate Punchline stack accumulation during combat |

---

## Planar Ornaments

### Celestial Differentiator (`planars/celestial_differentiator.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: +60% CRIT Rate for 1 attack when CRIT DMG ≥ 120% | **NOT MODELED** (single-attack window negligible in static sim) | Flag after ult; apply and consume on next hit |

### Sprightly Vonwacq (`planars/sprightly_vonwacq.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 2p: 40% action advance on battle enter (if SPD ≥ 120) | **NOT MODELED** (same AV-init timing problem as PT A4) | Re-push initial AV entry after `on_battle_start` |

### Tengoku Handout (`planars/tengoku_livestream.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | 4p: +32% CRIT DMG for 3 turns when 3+ SP consumed in one turn | Modeled as **50% uptime → +16% CRIT DMG permanently** | Track SP consumption per turn; apply and expire buff |

### Bone Collection's Serene Demesne (`planars/bone_collections_serene_demesne.rs`)
| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | +28% CRIT DMG when wearer HP ≥ 50% | Threshold evaluated **once at setup** using initial HP | Re-evaluate each turn based on current HP |

---

## Simulator Core (`simulator.rs` / `damage.rs`)

| # | What | Approximation | Fix |
|---|------|--------------|-----|
| 1 | Toughness damage values | Hardcoded: Basic=10, Skill=20, Ult=30, FUA/TalentProc=10 | Read from character ability data |
| 2 | Enemy level for ally DEF mitigation | Hardcoded to **95** | Use actual wave enemy level |
| 3 | Generic enemy fallback attack | **80% enemy ATK flat damage** | Add full enemy kits |
| 4 | Broken status damage multiplier | Enemy not broken → 0.9× multiplier on attacker (hardcoded in `damage.rs`) | Dynamic based on Break Type |
| 5 | 50% chance mechanics everywhere | All RNG replaced with **deterministic alternating flips** | Introduce seeded RNG for stochastic sim mode |
| 6 | No DoT tick damage | Burn/Bleed/Shock/Wind Shear debuffs are not ticking damage per turn | Add DoT tick at end of each enemy turn |
| 7 | No action advance from planars/relics | Eagle 4p, Vonwacq, PT A4 — all skipped for same reason | Modify `run_simulation` to allow `on_battle_start` hooks to adjust initial AV |
| 8 | Shield mechanics | Shields absorb damage in `apply_damage_to_ally` but **shield stacking cap** (300% of skill shield) is not enforced | Add cap logic per shield source |
| 9 | No debuff cleanse modeled | Souldragon cleanse, Bailu dispel, etc. are logged but debuffs are not actually removed | Implement cleanse in the relevant hooks |
| 10 | Targeting is always single deterministic | Enemies always target highest-aggro ally; attackers always target first alive enemy | Support random targeting or priority rules |

---

## How to Fix Common Patterns

**Battle-start action advance** (affects PT A4, Vonwacq, Eagle): In `run_simulation` (`simulator.rs`), after calling `dispatch_on_battle_start`, look for `_battle_start_advance_pct` in each member's stacks and re-push their AV queue entry:
```rust
for i in 0..state.team.len() {
    if let Some(&adv) = state.team[i].stacks.get("_battle_start_advance_pct") {
        if adv > 0.0 {
            let spd = effective_spd(&state.team[i]);
            let base_av = 10000.0 / spd;
            // Replace initial entry with advanced one — need to drain and re-push
        }
    }
}
```

**True 50% RNG**: Add a deterministic PRNG seeded per simulation run:
```rust
pub prng_state: u64, // in SimState
```
Then replace all flip-based patterns with `lcg_next(&mut state.prng_state) < 0.5`.
