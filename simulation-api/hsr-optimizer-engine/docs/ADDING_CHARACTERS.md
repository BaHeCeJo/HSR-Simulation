# Adding a Character

## Steps

1. Add the character's kit ID constant to `src/ids.rs`:
   ```rust
   pub const MY_CHAR_ID: &str = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx";
   ```

2. Create `src/characters/my_char.rs` using the template below.

3. In `src/characters/mod.rs`:
   - Add `pub mod my_char;` at the top.
   - Add `ids::MY_CHAR_ID => my_char::$fn_name($state, $idx),` to the `dispatch!` macro.
   - Add `ids::MY_CHAR_ID => my_char::on_before_action(...)` to `dispatch_on_before_action`.
   - Add `ids::MY_CHAR_ID => my_char::on_after_action(...)` to `dispatch_on_after_action`.
   - Add the same line to every other manual dispatch function (on_global_debuff, on_enemy_turn_start, on_enemy_action, on_ally_action).
   - If the character has a break mechanic: add to `dispatch_on_break`.

---

## Hook Reference

| Hook | When it fires | Typical use |
|---|---|---|
| `on_battle_start` | Once at battle start, before any turn | Set `max_energy`, apply minor traces, initialize stacks |
| `on_turn_start` | Start of this character's turn, before action choice | Per-turn energy regen, stack decay |
| `on_before_action` | After action is chosen, before damage is dealt; inside the **buffs snapshot window** | Mutate `action.multiplier`, add temporary `buffs.*` boosts, consume HP |
| `on_after_action` | After damage is dealt and enemy HP updated; still inside the buffs snapshot window | Deal secondary hits (adjacent, FUP), fix SP/energy, apply post-hit debuffs, add charges |
| `on_ult` | Called by the simulator when `energy >= max_energy` (or `_ult_ready` stack is set) | Set `_ult_handled = 1.0` to suppress default ult damage; deal custom AoE/multi-hit |
| `on_break` | When this character causes a Weakness Break on an enemy | A2 Bug (Silver Wolf), break-triggered effects |
| `on_global_debuff` | Every time ANY debuff is applied to ANY enemy | Acheron slash stacks, stack counters on ally debuffs |
| `on_enemy_turn_start` | Start of each enemy's turn | Decrement custom turn counters |
| `on_enemy_action` | After each enemy attacks | Charge gain from taking damage (Blade), per-hit tracking |
| `on_ally_action` | When any OTHER ally acts | E2 reactions, passive ally follow-ups |

---

## Buffs Snapshot Rule

The simulator takes a snapshot of `state.team[idx].buffs` **before** `on_before_action` and restores it **after** `on_after_action`. This means:

- **Temporary (action-scoped) boosts**: Add to `buffs.*` in `on_before_action` — they auto-revert. No cleanup needed.
- **Permanent boosts**: Apply in `on_battle_start`, or directly modify `base_stats` (not `buffs`). Do NOT set permanent buffs inside `on_before_action`.
- **Energy, HP, stacks, enemy state**: These are NOT restored by the snapshot. Changes persist.

---

## State Storage

| What to store | Where |
|---|---|
| Per-character turn/charge/state counters | `state.team[idx].stacks` (HashMap<String, f64>) |
| Global combat counters (energy, tally, shared flags) | `state.stacks` (HashMap<String, f64>) |
| Active timed debuffs on enemies | `enemy.active_debuffs` via `effects::apply_enemy_debuff` or `effects::try_apply_enemy_debuff` |
| Active timed buffs on allies | `member.active_buffs` |

---

## Damage Formula Inputs

```
final_dmg = base_dmg × dmg_boost × weaken × def × res × vuln × mitigation × broken × expected_crit

base_dmg  = (action.multiplier + action.extra_multiplier/100) × total_stat + action.extra_dmg
total_stat = (base_stats[scaling_stat_id] + lc_base) × (1 + stat_percent/100)
```

- `action.multiplier` — main scaling coefficient (e.g. 1.50 = 150%)
- `action.extra_multiplier` — additive coefficient in % (e.g. 20.0 = +20% of stat, added to multiplier)
- `action.extra_dmg` — flat damage added to base_dmg before all multipliers (use for tally/fixed amounts)
- `action.scaling_stat_id` — `ids::CHAR_ATK_ID`, `ids::CHAR_HP_ID`, or `ids::CHAR_DEF_ID`

---

## Applying Debuffs to Enemies

```rust
// Guaranteed application, increments debuff_count
effects::apply_enemy_debuff(enemy, "key", StatusEffect {
    duration: 3,          // enemy turns before expiry
    value:    45.0,       // numeric magnitude (e.g. 45% DEF reduction)
    stat:     Some("DEF Reduction".to_string()),
    effects:  vec![],
});

// Chance-based (EHR vs Effect RES), also increments debuff_count on success
effects::try_apply_enemy_debuff(ehr, enemy, "key", StatusEffect { ... }, base_chance);
// base_chance: 1.0 = 100%, 1.2 = 120%
```

Stat strings recognized by the damage formula:
- `"DEF Reduction"` or `"DEF Shred"` — reduces effective DEF
- `"All RES"` — reduces all-element resistance
- `"Weakness RES"` — reduces resistance for the attacker's element (only when weakness exists)
- `"Vulnerability"` — additive vulnerability multiplier
- Any other stat string is stored but not consumed by the formula (use for tracking)

---

## Custom Ult Handling

Set `_ult_handled` to prevent the simulator from dealing default ult damage:
```rust
pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].energy = 5.0;  // post-ult energy residue
    // ... deal damage manually with damage::calculate_damage(...)
}
```

---

## SP / Energy Correction in on_after_action

The simulator performs SP and energy accounting between `on_before_action` and `on_after_action`:
- Basic: `skill_points += 1`, `energy += 20 * err_mult`
- Skill: `skill_points -= 1`, `energy += 30 * err_mult`

To correct this (e.g. Blade converting Basic→Enhanced Basic):
```rust
let err_mult = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
// Remove the Basic SP gain, add Enhanced Basic energy delta:
state.skill_points = (state.skill_points - 1).max(0);
state.team[idx].energy = (state.team[idx].energy + 10.0 * err_mult).min(max_energy);
```

---

## Character Template

```rust
use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

// ─── Constants ────────────────────────────────────────────────────────────────

// Use state.team[idx].stacks for per-character state
// Use state.stacks for global combat state (energy pools, tallies)

// ─── Hooks ───────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy        = 120.0;
    state.team[idx].buffs.atk_percent += 28.0;  // minor trace
    state.team[idx].buffs.crit_rate   += 12.0;  // minor trace
    state.team[idx].buffs.effect_res  += 10.0;  // minor trace
    // Initialize any stacks
    // state.team[idx].stacks.insert("my_stack".to_string(), 0.0);
}

pub fn on_turn_start(_state: &mut SimState, _idx: usize) {}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    target_idx: Option<usize>,
) {
    // Temporary buffs added here auto-revert after on_after_action.
    // Example: +20% DMG boost for this action only
    // state.team[idx].buffs.dmg_boost += 20.0;
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    // Deal secondary hits, apply debuffs, adjust stacks.
    if !matches!(action.action_type, ActionType::Basic | ActionType::Skill | ActionType::Ultimate) {
        return;
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    // For characters with non-standard ult damage:
    state.team[idx].stacks.insert("_ult_handled".to_string(), 1.0);
    state.team[idx].energy = 5.0;

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if alive.is_empty() { return; }

    let member = state.team[idx].clone();
    let ult_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       3.60,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 60.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };

    let mut total = 0.0f64;
    for &slot in &alive {
        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &ult_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            total += dmg;
        }
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[slot] = None;
        }
    }
    state.total_damage += total;

    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Ult AoE: {:.0} DMG", total));
}

pub fn on_break(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _action: &ActionParams,
    _target_idx: Option<usize>,
) {}
```

---

## Common Patterns

### HP-scaling character
```rust
// In on_before_action:
action.scaling_stat_id  = ids::CHAR_HP_ID.to_string();
action.multiplier       = 1.50;  // 150% Max HP
action.toughness_damage = 30.0;
```

### AoE follow-up inside on_after_action
```rust
let alive: Vec<usize> = state.enemies.iter().enumerate()
    .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
    .collect();
let member = state.team[idx].clone();
let fup_action = ActionParams {
    action_type: ActionType::FollowUp,
    scaling_stat_id: ids::CHAR_ATK_ID.to_string(),
    multiplier: 1.00,
    extra_multiplier: 0.0, extra_dmg: 0.0,
    toughness_damage: 10.0,
    inflicts_debuff: false, is_ult_dmg: false,
};
for &slot in &alive {
    let dmg = state.enemies[slot].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &fup_action))
        .unwrap_or(0.0);
    if dmg > 0.0 {
        if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
    }
    if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[slot] = None;
    }
}
```

### Adjacent hits (Blast)
```rust
// target_slot = main target index
let left  = if target_slot > 0 { Some(target_slot - 1) } else { None };
let right = if target_slot + 1 < state.enemies.len() { Some(target_slot + 1) } else { None };
for adj in [left, right].iter().flatten() {
    if state.enemies[*adj].as_ref().map_or(true, |e| e.hp <= 0.0) { continue; }
    let dmg = state.enemies[*adj].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &adj_action))
        .unwrap_or(0.0);
    if dmg > 0.0 {
        if let Some(e) = state.enemies[*adj].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
    }
    if state.enemies[*adj].as_ref().map_or(false, |e| e.hp <= 0.0) {
        state.enemies[*adj] = None;
    }
}
```

### Custom energy (Anaxa-style)
```rust
// on_battle_start:
state.team[idx].max_energy = f64::MAX; // disable normal energy bar
state.stacks.insert("my_energy".to_string(), 0.0);

// In on_before_action: prevent simulator from tracking energy
state.team[idx].energy = 0.0;

// Mark ult ready when custom energy is full:
if state.stacks["my_energy"] >= 140.0 {
    state.team[idx].stacks.insert("_ult_ready".to_string(), 1.0);
}
```
