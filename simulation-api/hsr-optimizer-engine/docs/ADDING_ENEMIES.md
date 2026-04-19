# Adding an Enemy

Enemy kits live in `src/enemies/`. Each enemy defines how it deals damage when it acts, plus optional battle-start and turn-start setup.

---

## Steps

1. Add the enemy's kit ID constant to `src/ids.rs`:
   ```rust
   pub const MY_ENEMY_ID: &str = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx";
   ```

2. Create `src/enemies/my_enemy.rs` using the template below.

3. In `src/enemies/mod.rs`:
   - Add `mod my_enemy;` at the top.
   - Add `ids::MY_ENEMY_ID => my_enemy::on_battle_start(state, e_idx),` to `dispatch_on_battle_start`.
   - Add `ids::MY_ENEMY_ID => my_enemy::on_turn_start(state, e_idx),` to `dispatch_on_turn_start`.
   - Add `ids::MY_ENEMY_ID => my_enemy::on_action(state, e_idx, target_ally_idx),` to `dispatch_on_action`.

---

## Hook Reference

| Hook | When it fires | Return type |
|---|---|---|
| `on_battle_start` | Once at battle start | `()` — mutate enemy state |
| `on_turn_start` | Start of each enemy turn | `()` — mutate enemy state |
| `on_action` | When the enemy performs its attack | `Option<(f64, String)>` — damage + log message, or `None` for generic fallback |

### `on_action` return values
- `Some((damage, log_message))` — the simulator applies this damage to the target ally via `apply_damage_to_ally`, which handles shield absorption, HP reduction, and ally `on_hit_taken` effects.
- `None` — the simulator uses a generic damage calculation fallback.

---

## Enemy State (`SimEnemy`)

Accessed via `state.enemies[e_idx].as_ref()` (read) or `.as_mut()` (write):
```
enemy.hp                    // current HP
enemy.max_hp                // max HP
enemy.toughness             // current toughness bar
enemy.max_toughness
enemy.is_broken             // in Weakness Break state
enemy.level                 // level (for DEF formula)
enemy.weaknesses            // Vec<String> of weak elements
enemy.resistance            // base all-element RES (0.0 = 0%, 0.2 = 20%)
enemy.elemental_res         // HashMap<String, f64> per-element RES override
enemy.vulnerability         // flat % vulnerability (additive)
enemy.dmg_reduction         // flat % damage reduction
enemy.debuff_count          // number of active debuffs
enemy.active_debuffs        // HashMap<String, StatusEffect>
enemy.active_buffs          // HashMap<String, StatusEffect>
enemy.base_stats            // HashMap<String, f64> — ENEMY_ATK_ID, etc.
enemy.kit_id                // matches the ID constant in ids.rs
enemy.instance_id           // unique runtime ID (for per-enemy stacks)
enemy.name                  // display name
```

---

## Ally State for Damage Calculation

To read target ally stats in `on_action`:
```rust
let target = state.team.get(target_ally_idx)?;
let def    = target.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(600.0);
let max_hp = target.max_hp;
```

---

## Damage Formula for Enemy Attacks

Enemy attacks typically use a simplified formula — the simulator does not run the full HSR damage formula for enemies. The standard pattern:
```rust
let enemy_atk = enemy.base_stats.get(ids::ENEMY_ATK_ID).copied().unwrap_or(500.0);
let target_def = target.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(600.0);

let base_dmg = enemy_atk * multiplier;
let enemy_lv  = enemy.level as f64;
let def_mult  = (enemy_lv * 10.0 + 200.0) / (target_def + enemy_lv * 10.0 + 200.0);
let damage    = (base_dmg * def_mult).floor();
```

---

## Template

```rust
//! My Enemy
//! @id xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
//!
//! Abilities:
//!   - Smash: Deals Physical DMG (180% ATK) to a single target.
//!   - Phase 2 (HP < 50%): Smash becomes Crush (250% ATK).

use crate::ids;
use crate::models::SimState;

/// Attack action — returns `Some((damage, log))` or `None` if no valid target.
pub fn on_action(
    state: &SimState,
    e_idx: usize,
    target_ally_idx: usize,
) -> Option<(f64, String)> {
    let enemy  = state.enemies[e_idx].as_ref()?;
    let target = state.team.get(target_ally_idx)?;
    if target.is_downed { return None; }

    let enemy_atk  = enemy.base_stats.get(ids::ENEMY_ATK_ID).copied().unwrap_or(500.0);
    let target_def = target.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(600.0);

    // Phase 2 at HP < 50%
    let mult = if enemy.hp / enemy.max_hp < 0.5 { 2.5 } else { 1.8 };
    let ability = if enemy.hp / enemy.max_hp < 0.5 { "Crush" } else { "Smash" };

    let base_dmg = enemy_atk * mult;
    let lv       = enemy.level as f64;
    let def_mult = (lv * 10.0 + 200.0) / (target_def + lv * 10.0 + 200.0);
    let damage   = (base_dmg * def_mult).floor();

    let log = format!("{} (Physical) on {} -> {:.0} DMG", ability, target.name, damage);
    Some((damage, log))
}

/// Battle start — initialize enemy-specific stacks or modify base stats.
pub fn on_battle_start(state: &mut SimState, e_idx: usize) {
    // Example: give this enemy a phase-2 flag
    // if let Some(e) = state.enemies[e_idx].as_mut() {
    //     e.active_debuffs.insert("phase".to_string(), ...);
    // }
}

/// Turn start — triggered per-turn before the enemy attacks.
pub fn on_turn_start(_state: &mut SimState, _e_idx: usize) {}
```

---

## Common Enemy Patterns

### Enemy that applies a debuff to its target
```rust
pub fn on_action(state: &SimState, e_idx: usize, target_ally_idx: usize) -> Option<(f64, String)> {
    let enemy  = state.enemies[e_idx].as_ref()?;
    let target = state.team.get(target_ally_idx)?;
    if target.is_downed { return None; }
    // ... calculate damage ...
    // Debuffs on allies are applied by the simulator after on_action returns,
    // or can be handled via on_enemy_action hooks in characters.
    Some((damage, log))
}
```

### Enemy with multiple targets (AoE)
```rust
// on_action returns a single (damage, log) for one target.
// For AoE, return the main target damage and handle the rest in on_action by
// direct state mutation — but state is &SimState (immutable) in on_action.
// Workaround: use on_turn_start which receives &mut SimState.
pub fn on_turn_start(state: &mut SimState, e_idx: usize) {
    // AoE damage to all alive allies
    for ally in state.team.iter_mut() {
        if !ally.is_downed {
            ally.hp = (ally.hp - 500.0).max(0.0);
        }
    }
}
// Return None from on_action so the simulator skips the default single-target hit.
pub fn on_action(_state: &SimState, _e_idx: usize, _target: usize) -> Option<(f64, String)> {
    None // handled in on_turn_start
}
```

### Enemy that self-buffs at turn start
```rust
pub fn on_turn_start(state: &mut SimState, e_idx: usize) {
    if let Some(e) = state.enemies[e_idx].as_mut() {
        // Example: regenerate toughness
        e.toughness = (e.toughness + 10.0).min(e.max_toughness);
        // Example: apply self-buff
        e.vulnerability -= 10.0; // temporarily reduces vulnerability
    }
}
```
