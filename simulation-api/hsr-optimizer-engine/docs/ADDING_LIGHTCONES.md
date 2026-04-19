# Adding a Lightcone

## Steps

1. Add the LC ID constant to `src/ids.rs`:
   ```rust
   pub const LC_MY_LC_ID: &str = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx";
   ```

2. Create `src/lightcones/my_lc.rs` using the template below.

3. In `src/lightcones/mod.rs`:
   - Add `mod my_lc;` at the top.
   - Add `ids::LC_MY_LC_ID => my_lc::on_battle_start(state, idx),` to `dispatch_on_battle_start`.
   - Add the same to `dispatch_on_before_action` and `dispatch_on_after_action`.

---

## Hook Reference

| Hook | When it fires |
|---|---|
| `on_battle_start` | Once at battle start — apply permanent passive bonuses (CRIT, stat%, etc.) |
| `on_before_action` | Before each action, inside the **buffs snapshot window** — apply conditional bonuses |
| `on_after_action` | After each action — apply debuffs to enemies, set per-turn windows, update stacks |

The simulator calls each LC hook immediately **after** the matching character hook, so LC buffs stack on top of character buffs correctly.

---

## Superimposition Table Pattern

LC bonuses scale with superimposition (1–5). Use a table:
```rust
const CR_TABLE:  [f64; 5] = [12.0, 14.0, 16.0, 18.0, 20.0];
const DMG_TABLE: [f64; 5] = [24.0, 28.0, 32.0, 36.0, 40.0];

#[inline]
fn si_idx(si: i32) -> usize { ((si - 1).clamp(0, 4)) as usize }

// Usage:
let si  = state.team[idx].lightcone.superimposition;
let val = CR_TABLE[si_idx(si)];
```

---

## Buffs Snapshot Rule

Same as characters: any `buffs.*` change made in `on_before_action` is automatically reverted after `on_after_action`. No manual cleanup needed for per-action bonuses.

Permanent bonuses (like base CRIT DMG) must go in `on_battle_start`.

---

## Applying Debuffs from an LC

Use `effects::apply_enemy_debuff` for guaranteed debuffs (e.g. Mirage Fizzle):
```rust
effects::apply_enemy_debuff(enemy, "mirage_fizzle", StatusEffect {
    duration: 1,    // expires after 1 enemy turn tick
    value:    0.0,  // no numeric value needed for tracking debuffs
    stat:     None,
    effects:  vec![],
});
```

---

## Template

```rust
//! My Lightcone  (Path, ID: xxxxxxxx-…)
//!
//! SI 1/2/3/4/5:
//!   CRIT DMG  +36 / 42 / 48 / 54 / 60 %
//!   DMG       +24 / 28 / 32 / 36 / 40 %

use crate::models::{ActionParams, ActionType, SimState};

const CD_TABLE:  [f64; 5] = [36.0, 42.0, 48.0, 54.0, 60.0];
const DMG_TABLE: [f64; 5] = [24.0, 28.0, 32.0, 36.0, 40.0];

#[inline]
fn si_idx(si: i32) -> usize { ((si - 1).clamp(0, 4)) as usize }

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    let si = state.team[idx].lightcone.superimposition;
    state.team[idx].buffs.crit_dmg += CD_TABLE[si_idx(si)];
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    // Add conditional bonuses here — they auto-revert after the action.
    if matches!(action.action_type, ActionType::Ultimate | ActionType::FollowUp) {
        let si = state.team[idx].lightcone.superimposition;
        state.team[idx].buffs.dmg_boost += DMG_TABLE[si_idx(si)];
    }
}

pub fn on_after_action(
    _state: &mut SimState,
    _idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
```

---

## Common LC Patterns

### Permanent stat bonus only
```rust
pub fn on_battle_start(state: &mut SimState, idx: usize) {
    let si = state.team[idx].lightcone.superimposition;
    state.team[idx].buffs.crit_rate += [8.0, 9.0, 10.0, 11.0, 12.0][si_idx(si)];
}
// on_before_action and on_after_action → empty stubs
```

### Conditional DMG boost vs debuffed targets
```rust
pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    let has_debuff = target_idx
        .and_then(|t| state.enemies.get(t))
        .and_then(|s| s.as_ref())
        .map(|e| !e.active_debuffs.is_empty())
        .unwrap_or(false);
    if has_debuff {
        let si = state.team[idx].lightcone.superimposition;
        state.team[idx].buffs.dmg_boost += DMG_TABLE[si_idx(si)];
    }
}
```

### Post-hit debuff on enemy
```rust
pub fn on_after_action(
    state: &mut SimState,
    _idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    if !matches!(action.action_type, ActionType::Basic | ActionType::Skill | ActionType::Ultimate) {
        return;
    }
    if let Some(t) = target_idx {
        if let Some(enemy) = state.enemies.get_mut(t).and_then(|s| s.as_mut()) {
            effects::apply_enemy_debuff(enemy, "lc_debuff_key", StatusEffect {
                duration: 2, value: 0.0, stat: None, effects: vec![],
            });
        }
    }
}
```
