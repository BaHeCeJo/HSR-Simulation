# Adding a Planar Ornament Set

Planar ornament sets (Sphere + Rope) live in `src/planars/`. They always come as a 2-piece pair.

---

## Steps

1. Create `src/planars/my_set.rs` using the template below.

2. In `src/planars/mod.rs`:
   - Add `mod my_set;` in the mod list at the top.
   - Add a call to the set in `apply_set_bonuses`:
     ```rust
     my_set::apply(member, &relics);
     ```
     OR, if the set has a team-wide bonus triggered at setup, add to the `apply_team_set_bonuses` function:
     ```rust
     my_set::apply_team(team, relic_lists);
     ```

3. Add the ornament set ID to `relics/mod.rs` in two places:
   - The `ORNAMENT_SETS` array in `all_relic_configs()`:
     ```rust
     "my_set_2p",
     ```
   - The `config_to_relics` match arm:
     ```rust
     "my_set_2p" => ("my_set_internal_id", "my_set_internal_id"),
     ```
   - The `ornament_display` match arm:
     ```rust
     "my_set_2p" => "My Set Display Name 2p",
     ```
   - The `ORNAMENT_CODES` array in `all_set_combos()`.

4. The internal ID string (e.g. `"my_set_internal_id"`) must match what TypeScript sends as `relic.set_id` for the Sphere and Rope pieces.

---

## How Planars Work

The `apply_set_bonuses` function in `planars/mod.rs` calls each planar set's `apply` function once per character during setup. It counts how many pieces of the set the character has (always 0 or 2 for planars) and passes that count.

Some sets have a **conditional** (e.g. SPD threshold for Space Sealing, Fleet's SPD check). These conditions are evaluated at setup using the character's already-computed stats — so SPD from Musketeer 4p and feet main stat are already applied.

Some sets have **team-wide** bonuses (e.g. Fleet ATK%): these are handled in `apply_team_set_bonuses` which runs after all per-character bonuses are set.

---

## Checking SPD Threshold

The character's effective SPD at setup time:
```rust
let base_spd = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
let spd = base_spd * (1.0 + member.buffs.speed_percent / 100.0);
if spd >= 120.0 { /* threshold met */ }
```

---

## Templates

### Simple flat bonus (no condition)
```rust
//! My Planar Set
//!
//! (2p) ATK +12%
//!      Ult DMG +15%

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "my_set_internal_id";

pub fn apply(member: &mut TeamMember, relics: &[IncomingRelic]) {
    let count = relics.iter().filter(|r| r.set_id == SET_ID).count();
    if count < 2 { return; }

    member.buffs.atk_percent    += 12.0;
    member.buffs.ult_dmg_boost  += 15.0;
}
```

### Conditional bonus (SPD threshold — Space Sealing style)
```rust
//! My Planar Set
//!
//! (2p) ATK +12%
//!      If SPD >= 120: ATK +12% more

use crate::ids;
use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "my_set_internal_id";

pub fn apply(member: &mut TeamMember, relics: &[IncomingRelic]) {
    let count = relics.iter().filter(|r| r.set_id == SET_ID).count();
    if count < 2 { return; }

    member.buffs.atk_percent += 12.0;

    let base_spd = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    let spd = base_spd * (1.0 + member.buffs.speed_percent / 100.0);
    if spd >= 120.0 {
        member.buffs.atk_percent += 12.0;
    }
}
```

### Team bonus (Fleet-style: ATK% to all allies if any wearer meets threshold)
```rust
//! My Planar Set
//!
//! (2p) Max HP +12%
//!      When wearer SPD >= 120: all allies ATK +8%

use crate::ids;
use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "my_set_internal_id";

pub fn apply_per_wearer(member: &mut TeamMember) {
    member.buffs.hp_percent += 12.0;
}

pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_qualifies = (0..team.len()).any(|i| {
        if !relic_lists[i].iter().any(|r| r.set_id == SET_ID) { return false; }
        let base = team[i].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        let spd  = base * (1.0 + team[i].buffs.speed_percent / 100.0);
        spd >= 120.0
    });
    if any_qualifies {
        for member in team.iter_mut() {
            member.buffs.atk_percent += 8.0;
        }
    }
}
```

### CRIT conditional (Rutilant Arena / Inert Salsotto style)
```rust
//! My Planar Set
//!
//! (2p) CRIT Rate +8%
//!      If CRIT Rate >= 70%: Basic ATK and Skill DMG +20%

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "my_set_internal_id";

pub fn apply(member: &mut TeamMember, relics: &[IncomingRelic]) {
    let count = relics.iter().filter(|r| r.set_id == SET_ID).count();
    if count < 2 { return; }

    member.buffs.crit_rate += 8.0;

    if member.buffs.crit_rate >= 70.0 {
        member.buffs.basic_atk_dmg_boost += 20.0;
        member.buffs.skill_dmg_boost     += 20.0;
    }
}
```

### Nihility path bonus (Penacony style)
```rust
//! My Planar Set
//!
//! (2p) Effect HIT Rate +10%
//!      For each Nihility ally (including wearer): DMG +6% (max +24%)

use crate::models::{IncomingRelic, TeamMember};

const SET_ID: &str = "my_set_internal_id";

pub fn apply(member: &mut TeamMember, relics: &[IncomingRelic], team: &[TeamMember]) {
    let count = relics.iter().filter(|r| r.set_id == SET_ID).count();
    if count < 2 { return; }

    member.buffs.effect_hit_rate += 10.0;

    let nihility_count = team.iter().filter(|m| m.path == "Nihility").count().min(4);
    member.buffs.dmg_boost += nihility_count as f64 * 6.0;
}
```
*(Note: if `apply` needs the full team, adjust the signature and call site in planars/mod.rs accordingly.)*

---

## Planars mod.rs apply_set_bonuses call pattern

Most planars use:
```rust
my_set::apply(member, relics);
```

If the set needs the full team (path count, any-wearer checks), it goes in `apply_team_set_bonuses` instead.
