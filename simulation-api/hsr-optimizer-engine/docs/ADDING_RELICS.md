# Adding a Relic Set

Relic sets (4-piece cavity + 2-piece) live in `src/relics/`. Planar ornament sets (Sphere + Rope) live in `src/planars/`. This file covers the 4-piece cavity relic sets.

---

## Steps

1. Create `src/relics/my_set.rs` using the template below.

2. In `src/relics/mod.rs`:
   - Add `mod my_set;` in the mod list at the top.
   - Add the set in `apply_set_bonuses`:
     ```rust
     apply_set!("my_set_internal_id", my_set);
     ```
   - The internal ID string must exactly match what the TypeScript server sends as `relic.set_id`.

3. If the set has **team-wide** effects (e.g. Messenger SPD, Sacerdos CRIT DMG):
   - Add `pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>])` to the file.
   - Call it in `apply_team_set_bonuses`.

4. If the set has **simulation-time** effects (conditional buffs, turn windows, on-hit triggers):
   - Add the logic to the relevant hook in `relics/mod.rs`:
     - `apply_turn_start_effects` — decrement windows each turn start
     - `apply_action_conditional_buffs` — conditional buffs per action (inside snapshot window)
     - `on_action_used` — set windows/team buffs after each action
     - `on_attack_hit` — triggered when the wearer lands a hit
     - `on_hit_taken` — triggered when the wearer takes damage
     - `on_enemy_killed` — triggered when any enemy dies
     - `on_follow_up_start/hit/end` — triggered during follow-up attack sequences

5. Add the set to `RELIC_SETS` in `relics/mod.rs` so the optimizer includes it in search:
   ```rust
   ("My Set 4p", "my_set_internal_id", 4),
   ("My Set 2p", "my_set_internal_id", 2),
   ```

---

## Hook Reference

### Simple static bonus (most sets)

Sets that only grant flat stat bonuses at setup use a single `apply` function:
```rust
pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 { /* 2p bonus */ }
    if count >= 4 { /* 4p bonus */ }
}
```
Called once during character setup in `apply_set_bonuses`. Mutations go directly to `member.buffs` or `member.base_stats`.

### Simulation-time bonuses

Sets that activate during combat use the `member.stacks` map as a flag/counter:
- A **window** flag (e.g. `"band_skill_window"`) counts remaining turns.
- The window is **set** in `on_action_used` after the triggering action.
- The window is **decremented** each turn in `apply_turn_start_effects`.
- The **buff itself** is applied in `apply_action_conditional_buffs` (inside the buffs snapshot — auto-reverts).

---

## Buffs Available

```rust
member.buffs.atk_percent       += v;   // ATK %
member.buffs.def_percent       += v;   // DEF %
member.buffs.hp_percent        += v;   // HP %
member.buffs.crit_rate         += v;   // CRIT Rate %
member.buffs.crit_dmg          += v;   // CRIT DMG %
member.buffs.dmg_boost         += v;   // All-DMG %
member.buffs.basic_atk_dmg_boost += v; // Basic ATK DMG % only
member.buffs.skill_dmg_boost   += v;   // Skill DMG % only
member.buffs.ult_dmg_boost     += v;   // Ult DMG % only
member.buffs.follow_up_dmg_boost += v; // Follow-up DMG % only
member.buffs.def_ignore        += v;   // DEF ignore %
member.buffs.def_reduction     += v;   // DEF reduction (attacker-side)
member.buffs.res_pen           += v;   // RES penetration %
member.buffs.break_effect      += v;   // Break Effect % (temp via stacks)
member.buffs.speed_percent     += v;   // SPD %
member.buffs.energy_regen_rate += v;   // ERR %
member.buffs.outgoing_healing  += v;   // Outgoing Healing %
member.buffs.effect_hit_rate   += v;   // Effect HIT Rate %
member.buffs.effect_res        += v;   // Effect RES %

// Flat additions (add to base_stats, not buffs):
*member.base_stats.entry(ids::CHAR_ATK_ID.to_string()).or_insert(0.0) += v;
*member.base_stats.entry(ids::CHAR_HP_ID.to_string()).or_insert(0.0)  += v;
*member.base_stats.entry(ids::CHAR_SPD_ID.to_string()).or_insert(0.0) += v;
*member.base_stats.entry(ids::CHAR_BE_ID.to_string()).or_insert(0.0)  += v;
```

---

## Templates

### Simple static set (no simulation hooks)

```rust
//! My Relic Set
//!
//! (2p) ATK +12%
//! (4p) CRIT Rate +8% | Skill DMG +20%

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 {
        member.buffs.atk_percent += 12.0;
    }
    if count >= 4 {
        member.buffs.crit_rate       += 8.0;
        member.buffs.skill_dmg_boost += 20.0;
    }
}
```

### Set with a turn window (e.g. Band of Sizzling Thunder)

```rust
//! My Relic Set
//!
//! (2p) Lightning DMG +10%
//! (4p) ATK +20% for 1 turn after using Skill

use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Lightning" {
        member.buffs.dmg_boost += 10.0;
    }
    // 4p effect is applied dynamically via apply_action_conditional_buffs / on_action_used
}
```

Then in `relics/mod.rs`:

**`apply_action_conditional_buffs`** — apply the buff while the window is active:
```rust
if count_set(&relics, "my_set_internal_id") >= 4 {
    if member.stacks.get("my_set_window").copied().unwrap_or(0.0) > 0.0 {
        member.buffs.atk_percent += 20.0; // auto-reverts after action (snapshot)
    }
}
```

**`on_action_used`** — set the window after the triggering action:
```rust
if is_skill && count_set(&relics, "my_set_internal_id") >= 4 {
    team[wearer_idx].stacks.insert("my_set_window".to_string(), 1.0);
}
```

**`apply_turn_start_effects`** — decrement the window:
```rust
if let Some(w) = member.stacks.get_mut("my_set_window") {
    if *w > 0.0 { *w -= 1.0; }
}
```

### Set with kill stacking (e.g. Sigonia)

```rust
// apply_action_conditional_buffs:
if count_set(&relics, "my_set_internal_id") >= 2 {
    let stacks = member.stacks.get("my_stacks").copied().unwrap_or(0.0).min(10.0);
    member.buffs.crit_dmg += stacks * 4.0;  // auto-reverts
}

// on_enemy_killed:
for member in team.iter_mut() {
    if count_set(&member.relics, "my_set_internal_id") >= 2 {
        let s = member.stacks.entry("my_stacks".to_string()).or_insert(0.0);
        *s = (*s + 1.0).min(10.0);
    }
}
```

### Team-wide buff (e.g. Messenger SPD)

```rust
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let has_4p = (0..team.len()).any(|i| {
        relic_lists[i].iter().filter(|r| r.set_id == "my_set_internal_id").count() >= 4
    });
    if has_4p {
        for member in team.iter_mut() {
            member.buffs.atk_percent += 8.0;  // or whatever the team buff is
        }
    }
}
```
