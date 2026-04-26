# HSR Engine — Implementation Reference

> **Purpose**: Single-file lookup for implementing characters, lightcones, relic sets, planar ornament sets, and enemies.  
> **Keep this file updated** whenever a struct field is added, a new pattern is established, or an architectural rule changes.

---

## Table of Contents

1. [Wiring Checklist](#wiring-checklist)
2. [Key Architectural Rules](#key-architectural-rules)
3. [Models — Current Field Reference](#models--current-field-reference)
4. [Damage Formula](#damage-formula)
5. [Character Implementation](#character-implementation)
6. [Lightcone Implementation](#lightcone-implementation)
7. [Relic Set Implementation](#relic-set-implementation)
8. [Planar Ornament Implementation](#planar-ornament-implementation)
9. [Enemy Implementation](#enemy-implementation)
10. [Common Patterns Library](#common-patterns-library)

---

## Wiring Checklist

### New Character
- [ ] Add `pub const MY_CHAR_ID: &str = "...";` to `src/ids.rs`
- [ ] Create `src/characters/my_char.rs`
- [ ] In `src/characters/mod.rs`:
  - [ ] `pub mod my_char;` at top
  - [ ] Arm in `dispatch!` macro (7 dispatch functions total):
    - `dispatch!` macro → `on_battle_start`, `on_turn_start`, `on_ult`
    - `dispatch_on_before_action`
    - `dispatch_on_after_action`
    - `dispatch_on_global_debuff`
    - `dispatch_on_enemy_turn_start`
    - `dispatch_on_enemy_action`
    - `dispatch_on_ally_action`
  - [ ] (Optional) `dispatch_on_break` if character has break mechanic

### New Lightcone
- [ ] Add `pub const LC_MY_LC_ID: &str = "...";` to `src/ids.rs`
- [ ] Create `src/lightcones/my_lc.rs`
- [ ] In `src/lightcones/mod.rs`: add `mod my_lc;` + dispatch arms for `on_battle_start`, `on_before_action`, `on_after_action`

### New Relic Set
- [ ] Create `src/relics/my_set.rs`
- [ ] In `src/relics/mod.rs`: `mod my_set;` + `apply_set!("set_id", my_set)` in `apply_set_bonuses`
- [ ] Add to `RELIC_SETS` array for optimizer search
- [ ] (Optional) `apply_team_set_bonuses` for team-wide effects
- [ ] (Optional) hooks: `apply_turn_start_effects`, `apply_action_conditional_buffs`, `on_action_used`, `on_attack_hit`, `on_enemy_killed`

### New Planar Set
- [ ] Create `src/planars/my_set.rs`
- [ ] In `src/planars/mod.rs`: `mod my_set;` + call in `apply_set_bonuses` (or `apply_team_set_bonuses`)
- [ ] In `src/relics/mod.rs`: add to `ORNAMENT_SETS`, `config_to_relics`, `ornament_display`, `ORNAMENT_CODES`

### New Enemy
- [ ] Add `pub const MY_ENEMY_ID: &str = "...";` to `src/ids.rs`
- [ ] Create `src/enemies/my_enemy.rs`
- [ ] In `src/enemies/mod.rs`: `mod my_enemy;` + dispatch arms for `on_battle_start`, `on_turn_start`, `on_action`

---

## Key Architectural Rules

### Buffs Snapshot
The simulator snapshots `state.team[idx].buffs` **before** `on_before_action` and **restores** it **after** `on_after_action`.

- **Temporary (per-action)**: Add to `buffs.*` in `on_before_action` → auto-reverts. No cleanup.
- **Permanent**: Set in `on_battle_start`, or write to `base_stats`. Never in `on_before_action`.
- **Observer buffs** (in `on_ally_action`): Only the *acting* ally's buffs are snapshotted. Changes to the observer's `buffs.*` in `on_ally_action` **are permanent** until you undo them.

### Self-Buff Persistence Pattern
If a character needs a buff that persists across multiple of their own turns (e.g. Cipher's +30% ATK after Skill lasting 2 turns):
- Track remaining turns in `state.stacks["my_char_buff_rem"]`
- In `on_before_action`: check the counter → if > 0, apply buff to `buffs.*` temporarily AND decrement the counter
- The buff is active for that action; the counter persists until it hits 0

### State Keys
| What to store | Where | Key type |
|---|---|---|
| Per-character counters, flags | `state.team[idx].stacks` | `&'static str` (no `format!()`) |
| Global, cross-character, or dynamic string keys | `state.stacks` | `String` |
| Per-target counters (keyed by enemy slot) | `state.stacks` | `format!("key_{slot}")` |

**Never** use `format!()` as a key in `state.team[idx].stacks`. Only string literals allowed there.

### Enemy Cache Rule
**Always call `effects::recompute_enemy_caches(enemy)`** after manually inserting/removing from `enemy.active_debuffs` or `enemy.active_buffs`. The four cached fields (`cached_def_reduce`, `cached_all_res_reduce`, `cached_weakness_res_reduce`, `cached_vuln_bonus`) are only updated by this call.

### True DMG Pattern
True DMG bypasses CRIT, DMG%, DEF, and RES. Apply only: vulnerability × mitigation × broken.
```rust
fn apply_true_dmg(state: &mut SimState, attacker_idx: usize, enemy_slot: usize, amount: f64) {
    if let Some(enemy) = state.enemies[enemy_slot].as_mut() {
        let vuln   = 1.0 + (enemy.vulnerability + enemy.cached_vuln_bonus) / 100.0;
        let mitig  = 1.0 - enemy.dmg_reduction / 100.0;
        let broken = if enemy.is_broken { 1.0 } else { 0.9 };
        let dmg = (amount * vuln * mitig * broken).floor();
        enemy.hp -= dmg;
        state.total_damage += dmg;
    }
}
```

### Memosprite / Summon Pattern (Castorice/Netherwing)
See `src/characters/castorice.rs`. In `simulator.rs`, a dedicated dispatch block before the normal character turn handler checks `entry.actor_id == ids::NETHERWING_ID`. Uses a generation counter (`"netherwing_gen"` in `state.stacks`) stored as `instance_id` in the AV queue entry to invalidate stale entries when the memosprite dies.

---

## Models — Current Field Reference

### `Buffs` struct (all fields, with defaults)

```rust
// Scaling stat %
atk_percent:           0.0   // ATK%
atk_flat:              0.0   // Flat ATK added after % scaling (added 2025-04)
def_percent:           0.0   // DEF%
hp_percent:            0.0   // HP%
speed_percent:         0.0   // SPD%
// Combat
crit_rate:             5.0   // CRIT Rate % (base 5)
crit_dmg:             50.0   // CRIT DMG % (base 50)
dmg_boost:             0.0   // All-DMG %
basic_atk_dmg_boost:   0.0   // Basic only
skill_dmg_boost:       0.0   // Skill only
ult_dmg_boost:         0.0   // Ultimate only
follow_up_dmg_boost:   0.0   // Follow-up only
def_ignore:            0.0   // DEF ignore %
def_reduction:         0.0   // DEF reduction (attacker-side)
extra_multiplier:      0.0   // extra_multiplier added to action.extra_multiplier
extra_dmg:             0.0   // flat addition to base_dmg
res_pen:               0.0   // RES penetration %
weaken:                0.0   // outgoing DMG penalty on attacker
break_efficiency:      0.0   // toughness reduction bonus %
break_effect:          0.0   // Break Effect % (temporary, added to base_stats BE in break calc)
// Utility
outgoing_healing:      0.0
effect_hit_rate:       0.0
effect_res:            0.0
energy_regen_rate:     0.0   // ERR %
incoming_dmg_reduction: 0.0  // % reduction to all incoming DMG
shield_effect:         0.0   // % boost to shield size
```

> **To add a new Buffs field**: (1) Add field to `Buffs` struct in `models.rs`, (2) add default value in `impl Default for Buffs`, (3) add usage in `damage.rs` if it affects the formula, (4) update this reference.

### `TeamMember` (key fields)
```
kit_id, name, element, path, level, eidolon
hp, max_hp, shield, is_downed
energy, max_energy
base_stats: StatMap            // UUID → f64 (ATK, DEF, HP, SPD, BE, etc.)
buffs: Buffs                   // snapshotted per-action
stacks: HashMap<&'static str, f64>    // character-local state (static str keys only)
turn_counters: HashMap<&'static str, i32>
lightcone: LightconeStats      // .id, .superimposition, .base_stats
relics: Vec<IncomingRelic>     // .set_id, .slot, .main_stat
abilities: Vec<IncomingAbility>
has_memo: bool                 // for memo-conditional set bonuses
is_fua: bool                   // for FUA-conditional set bonuses
```

### `SimEnemy` (key fields)
```
kit_id, instance_id, name, level
hp, max_hp
toughness, max_toughness, is_broken
weaknesses: Vec<String>
resistance: f64                // base all-element RES (0.0 = 0%)
elemental_res: HashMap<String, f64>   // per-element override
vulnerability: f64             // direct % (additive, modify directly)
dmg_reduction: f64
debuff_count: u32
active_debuffs: HashMap<String, StatusEffect>
active_buffs: HashMap<String, StatusEffect>
base_stats: StatMap
// Caches — always recompute after modifying debuffs/buffs:
cached_def_reduce: f64
cached_all_res_reduce: f64
cached_weakness_res_reduce: f64
cached_vuln_bonus: f64
```

### `ActionParams` (fields)
```
action_type: ActionType        // Basic | Skill | Ultimate | FollowUp | TalentProc | EnemyAttack
scaling_stat_id: String        // ids::CHAR_ATK_ID / CHAR_HP_ID / CHAR_DEF_ID
multiplier: f64                // main coefficient (1.5 = 150%)
extra_multiplier: f64          // additive % to multiplier (20.0 = +20%)
extra_dmg: f64                 // flat added to base_dmg before all multipliers
toughness_damage: f64
inflicts_debuff: bool          // triggers on_global_debuff dispatch if true
is_ult_dmg: bool               // marks as ult DMG for zone checks
```

### `StatusEffect` (fields)
```
duration: i32      // enemy turns before expiry
value: f64         // numeric magnitude (e.g. 45.0 for 45% DEF reduction)
stat: Option<String>
effects: Vec<StatChange>
```

Stat strings recognized by `recompute_enemy_caches`:
- `"DEF Reduction"` / `"DEF Shred"` → `cached_def_reduce`
- `"All RES"` → `cached_all_res_reduce`
- `"Weakness RES"` → `cached_weakness_res_reduce`
- `"Vulnerability"` → `cached_vuln_bonus`

---

## Damage Formula

```
final_dmg = base_dmg × dmg_boost × weaken × def × res × vuln × mitigation × broken × expected_crit

base_dmg   = (action.multiplier + action.extra_multiplier/100) × total_stat + action.extra_dmg
total_stat = (base_stats[scaling_stat_id] + lc_base) × (1 + stat_percent/100) + atk_flat
              ↑ atk_flat only applies to ATK-scaling (not HP/DEF scaling)

dmg_boost  = 1 + (buffs.dmg_boost + action_type_boost) / 100
def_mult   = (att_lv+20) / ((def_lv+20) × max(0, 1−def_ignore−def_reduction) + (att_lv+20))
res_mult   = clamp(1 − (res − res_pen), 0.10, 2.00)
vuln_mult  = 1 + (enemy.vulnerability + cached_vuln_bonus) / 100
mitig_mult = 1 − enemy.dmg_reduction / 100
broken_mult = 1.0 if broken, else 0.9
crit_mult  = 1 + clamp(crit_rate, 0, 1) × crit_dmg
```

**Break DMG** = `break_base_coeff(element) × level_mult(lv) × (0.5 + max_toughness/40) × (1+BE) × def × res × vuln × mitig × 0.9`  
Break coefficients: Physical/Fire=2.0, Wind=1.5, Ice/Lightning=1.0, Quantum/Imaginary=0.5

---

## Character Implementation

### Hook Signatures

```rust
pub fn on_battle_start(state: &mut SimState, idx: usize) {}
pub fn on_turn_start(state: &mut SimState, idx: usize) {}
pub fn on_before_action(state: &mut SimState, idx: usize, action: &mut ActionParams, target_idx: Option<usize>) {}
pub fn on_after_action(state: &mut SimState, idx: usize, action: &ActionParams, target_idx: Option<usize>) {}
pub fn on_ult(state: &mut SimState, idx: usize) {}
pub fn on_break(state: &mut SimState, idx: usize, enemy_slot: usize) {}
pub fn on_global_debuff(state: &mut SimState, idx: usize, source_idx: usize, enemy_idx: usize) {}
pub fn on_enemy_turn_start(state: &mut SimState, idx: usize, enemy_idx: usize) {}
pub fn on_enemy_action(state: &mut SimState, idx: usize, enemy_idx: usize) {}
pub fn on_ally_action(state: &mut SimState, idx: usize, source_idx: usize, action: &ActionParams, target_idx: Option<usize>) {}
```

### Minimal Template

```rust
use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy         = 120.0;
    state.team[idx].buffs.crit_rate   += 5.3;   // minor trace
    state.team[idx].buffs.atk_percent += 28.0;  // minor trace
    state.team[idx].buffs.effect_res  += 10.0;  // minor trace
}

pub fn on_turn_start(_state: &mut SimState, _idx: usize) {}

pub fn on_before_action(
    state: &mut SimState, idx: usize,
    action: &mut ActionParams, _target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Basic => {
            action.multiplier       = 1.00;
            action.toughness_damage = 30.0;
        }
        ActionType::Skill => {
            action.multiplier       = 2.10;
            action.toughness_damage = 60.0;
            action.inflicts_debuff  = true;
        }
        _ => {}
    }
}

pub fn on_after_action(
    state: &mut SimState, idx: usize,
    action: &ActionParams, target_idx: Option<usize>,
) {}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    if alive.is_empty() { return; }

    let member = state.team[idx].clone();
    let ult_action = ActionParams {
        action_type: ActionType::Ultimate,
        scaling_stat_id: ids::CHAR_ATK_ID.to_string(),
        multiplier: 2.40, extra_multiplier: 0.0, extra_dmg: 0.0,
        toughness_damage: 60.0, inflicts_debuff: false, is_ult_dmg: true,
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
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) { state.enemies[slot] = None; }
    }
    state.total_damage += total;
    let name = state.team[idx].name.clone();
    state.add_log(&name, format!("Ult AoE: {:.0}", total));
}

pub fn on_break(_state: &mut SimState, _idx: usize, _enemy_slot: usize) {}
pub fn on_global_debuff(_state: &mut SimState, _idx: usize, _source_idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_turn_start(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}
pub fn on_ally_action(_state: &mut SimState, _idx: usize, _source_idx: usize, _action: &ActionParams, _target_idx: Option<usize>) {}
```

### Custom Ult (`_ult_handled`)
Set `state.team[idx].stacks.insert("_ult_handled", 1.0)` to suppress the simulator's default ult damage path. Set `state.team[idx].energy = 5.0` for the residue energy after ult.

### Custom Energy Bar (Anaxa-style)
```rust
// on_battle_start:
state.team[idx].max_energy = f64::MAX;  // disable normal energy
state.stacks.insert("my_energy".to_string(), 0.0);

// In on_before_action: prevent normal energy accumulation
state.team[idx].energy = 0.0;

// Set ult ready flag when full:
if state.stacks.get("my_energy").copied().unwrap_or(0.0) >= 140.0 {
    state.team[idx].stacks.insert("_ult_ready", 1.0);
}
```

### SP / Energy Correction
```rust
// In on_after_action, if Basic was converted to Enhanced:
let err_mult = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
state.skill_points = (state.skill_points - 1).max(0); // remove Basic SP
state.team[idx].energy = (state.team[idx].energy + 10.0 * err_mult).min(max_energy);
```

---

## Lightcone Implementation

### Hook Signatures
```rust
pub fn on_battle_start(state: &mut SimState, idx: usize) {}
pub fn on_before_action(state: &mut SimState, idx: usize, action: &mut ActionParams, target_idx: Option<usize>) {}
pub fn on_after_action(state: &mut SimState, idx: usize, action: &ActionParams, target_idx: Option<usize>) {}
```

### Superimposition Table Pattern
```rust
const CR_TABLE: [f64; 5] = [12.0, 14.0, 16.0, 18.0, 20.0];

#[inline]
fn si_idx(si: i32) -> usize { ((si - 1).clamp(0, 4)) as usize }

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    let si = state.team[idx].lightcone.superimposition;
    state.team[idx].buffs.crit_rate += CR_TABLE[si_idx(si)];
}
```

### Conditional DMG Boost vs Debuffed Target
```rust
pub fn on_before_action(state: &mut SimState, idx: usize, action: &mut ActionParams, target_idx: Option<usize>) {
    let has_debuff = target_idx
        .and_then(|t| state.enemies.get(t)).and_then(|s| s.as_ref())
        .map(|e| !e.active_debuffs.is_empty()).unwrap_or(false);
    if has_debuff {
        state.team[idx].buffs.dmg_boost += 24.0;
    }
}
```

### Post-hit Debuff on Enemy
```rust
pub fn on_after_action(state: &mut SimState, _idx: usize, action: &ActionParams, target_idx: Option<usize>) {
    if !matches!(action.action_type, ActionType::Basic | ActionType::Skill | ActionType::Ultimate) { return; }
    if let Some(t) = target_idx {
        if let Some(enemy) = state.enemies.get_mut(t).and_then(|s| s.as_mut()) {
            effects::apply_enemy_debuff(enemy, "lc_debuff_key", StatusEffect {
                duration: 2, value: 0.0, stat: None, effects: vec![],
            });
        }
    }
}
```

---

## Relic Set Implementation

### Simple Static Bonus
```rust
// src/relics/my_set.rs
use crate::models::TeamMember;

pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 { member.buffs.atk_percent += 12.0; }
    if count >= 4 { member.buffs.crit_rate += 8.0; member.buffs.skill_dmg_boost += 20.0; }
}
```

### Set With a Turn Window (Band-style)
```rust
// relics/my_set.rs — apply for static part only:
pub fn apply(member: &mut TeamMember, count: usize) {
    if count >= 2 && member.element == "Lightning" { member.buffs.dmg_boost += 10.0; }
}

// In relics/mod.rs:

// apply_action_conditional_buffs (inside snapshot → auto-reverts):
if count_set(&relics, "set_id") >= 4 {
    if member.stacks.get("my_set_window").copied().unwrap_or(0.0) > 0.0 {
        member.buffs.atk_percent += 20.0;
    }
}

// on_action_used — set window after Skill:
if is_skill && count_set(&relics, "set_id") >= 4 {
    team[wearer_idx].stacks.insert("my_set_window", 1.0);
}

// apply_turn_start_effects — decrement:
if let Some(w) = member.stacks.get_mut("my_set_window") { if *w > 0.0 { *w -= 1.0; } }
```

### Kill-Stacking Set (Sigonia-style)
```rust
// apply_action_conditional_buffs:
let stacks = member.stacks.get("my_stacks").copied().unwrap_or(0.0).min(10.0);
member.buffs.crit_dmg += stacks * 4.0;  // auto-reverts

// on_enemy_killed:
for member in team.iter_mut() {
    if count_set(&member.relics, "set_id") >= 2 {
        let s = member.stacks.entry("my_stacks").or_insert(0.0);
        *s = (*s + 1.0).min(10.0);
    }
}
```

### Team-Wide Buff (Messenger-style)
```rust
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let has_4p = (0..team.len()).any(|i| {
        relic_lists[i].iter().filter(|r| r.set_id == "set_id").count() >= 4
    });
    if has_4p {
        for member in team.iter_mut() { member.buffs.speed_percent += 12.0; }
    }
}
```

---

## Planar Ornament Implementation

### Simple Flat Bonus
```rust
use crate::models::{IncomingRelic, TeamMember};
const SET_ID: &str = "my_planar_id";

pub fn apply(member: &mut TeamMember, relics: &[IncomingRelic]) {
    if relics.iter().filter(|r| r.set_id == SET_ID).count() < 2 { return; }
    member.buffs.atk_percent   += 12.0;
    member.buffs.ult_dmg_boost += 15.0;
}
```

### SPD Threshold (Space Sealing-style)
```rust
use crate::ids;
pub fn apply(member: &mut TeamMember, relics: &[IncomingRelic]) {
    if relics.iter().filter(|r| r.set_id == SET_ID).count() < 2 { return; }
    member.buffs.atk_percent += 12.0;
    let base_spd = member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    let spd = base_spd * (1.0 + member.buffs.speed_percent / 100.0);
    if spd >= 120.0 { member.buffs.atk_percent += 12.0; }
}
```

### CRIT Threshold (Rutilant-style)
```rust
pub fn apply(member: &mut TeamMember, relics: &[IncomingRelic]) {
    if relics.iter().filter(|r| r.set_id == SET_ID).count() < 2 { return; }
    member.buffs.crit_rate += 8.0;
    if member.buffs.crit_rate >= 70.0 {
        member.buffs.basic_atk_dmg_boost += 20.0;
        member.buffs.skill_dmg_boost     += 20.0;
    }
}
```

### Team Bonus (Fleet-style)
```rust
pub fn apply_team(team: &mut Vec<TeamMember>, relic_lists: &[Vec<IncomingRelic>]) {
    let any_qualifies = (0..team.len()).any(|i| {
        if !relic_lists[i].iter().any(|r| r.set_id == SET_ID) { return false; }
        let base = team[i].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
        base * (1.0 + team[i].buffs.speed_percent / 100.0) >= 120.0
    });
    if any_qualifies { for m in team.iter_mut() { m.buffs.atk_percent += 8.0; } }
}
```

---

## Enemy Implementation

### Hook Signatures
```rust
pub fn on_battle_start(state: &mut SimState, e_idx: usize) {}
pub fn on_turn_start(state: &mut SimState, e_idx: usize) {}
pub fn on_action(state: &SimState, e_idx: usize, target_ally_idx: usize) -> Option<(f64, String)> {}
```

`on_action` returns `Some((damage, log))` → simulator calls `apply_damage_to_ally`.  
`on_action` returns `None` → simulator uses generic fallback.

### Template
```rust
use crate::ids;
use crate::models::SimState;

pub fn on_action(state: &SimState, e_idx: usize, target_ally_idx: usize) -> Option<(f64, String)> {
    let enemy  = state.enemies[e_idx].as_ref()?;
    let target = state.team.get(target_ally_idx)?;
    if target.is_downed { return None; }

    let enemy_atk  = enemy.base_stats.get(ids::ENEMY_ATK_ID).copied().unwrap_or(500.0);
    let target_def = target.base_stats.get(ids::CHAR_DEF_ID).copied().unwrap_or(600.0);
    let mult = 1.80;
    let lv   = enemy.level as f64;
    let def_mult = (lv * 10.0 + 200.0) / (target_def + lv * 10.0 + 200.0);
    let dmg  = (enemy_atk * mult * def_mult).floor();

    Some((dmg, format!("Attack on {} -> {:.0} DMG", target.name, dmg)))
}

pub fn on_battle_start(_state: &mut SimState, _e_idx: usize) {}
pub fn on_turn_start(_state: &mut SimState, _e_idx: usize) {}
```

---

## Common Patterns Library

### Applying Debuffs to Enemies
```rust
// Guaranteed application (increments debuff_count, triggers on_global_debuff):
effects::apply_enemy_debuff(enemy, "key", StatusEffect {
    duration: 2, value: 45.0,
    stat: Some("DEF Reduction".to_string()), effects: vec![],
});

// Chance-based (EHR vs Effect RES):
effects::try_apply_enemy_debuff(ehr, enemy, "key", StatusEffect { ... }, base_chance);
// base_chance 1.0 = 100%, 1.2 = 120% base
```

### Single-Target Hit (in on_after_action)
```rust
if let Some(slot) = target_idx {
    if state.enemies[slot].as_ref().map_or(false, |e| e.hp > 0.0) {
        let member = state.team[idx].clone();
        let hit_action = ActionParams {
            action_type: ActionType::FollowUp,
            scaling_stat_id: ids::CHAR_ATK_ID.to_string(),
            multiplier: 1.20, extra_multiplier: 0.0, extra_dmg: 0.0,
            toughness_damage: 15.0, inflicts_debuff: false, is_ult_dmg: false,
        };
        let dmg = state.enemies[slot].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &hit_action)).unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
            state.total_damage += dmg;
        }
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) { state.enemies[slot] = None; }
    }
}
```

### AoE Hit
```rust
let alive: Vec<usize> = state.enemies.iter().enumerate()
    .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
    .collect();
let member = state.team[idx].clone();
let aoe = ActionParams { action_type: ActionType::FollowUp, scaling_stat_id: ids::CHAR_ATK_ID.to_string(),
    multiplier: 0.60, extra_multiplier: 0.0, extra_dmg: 0.0,
    toughness_damage: 10.0, inflicts_debuff: false, is_ult_dmg: false };
let mut total = 0.0f64;
for &slot in &alive {
    let dmg = state.enemies[slot].as_ref().map(|e| damage::calculate_damage(&member, e, &aoe)).unwrap_or(0.0);
    if dmg > 0.0 { if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; } total += dmg; }
    if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) { state.enemies[slot] = None; }
}
state.total_damage += total;
```

### Blast (Main + Adjacent)
```rust
fn adj_slots(slot: usize, enemies: &[Option<_>]) -> Vec<usize> {
    let mut v = Vec::new();
    if slot > 0 && enemies[slot-1].is_some() { v.push(slot-1); }
    if slot+1 < enemies.len() && enemies[slot+1].is_some() { v.push(slot+1); }
    v
}
// In hook: deal main hit to target_slot, then adj_action to adj_slots(target_slot, &state.enemies)
```

### HP-Scaling Character
```rust
// In on_before_action:
action.scaling_stat_id = ids::CHAR_HP_ID.to_string();
action.multiplier = 1.50;
// Note: atk_flat does NOT apply to HP-scaling (only to ATK-scaling)
```

### DEF-Scaling Character (Aventurine-style)
```rust
// In on_before_action:
action.scaling_stat_id = ids::CHAR_DEF_ID.to_string();
action.multiplier = 0.50;
// buffs.def_percent drives the scaling stat; atk_flat does not apply
```

### Tally → True DMG Pattern (Cipher-style)
```rust
// Accumulate tally: add (some_dmg × rate) to state.stacks["char_tally"]
// Discharge in Ult:
let tally = state.stacks.remove("char_tally").unwrap_or(0.0);
apply_true_dmg(&mut state, idx, target_slot, tally);
```

### E2 Vulnerability Ticking (per-enemy, turn-based)
```rust
// Apply in on_after_action (direct enemy.vulnerability modification):
enemy.vulnerability += 7.0;
state.stacks.insert(format!("char_e2_{slot}"), 2.0);  // 2 turns duration

// Tick in on_enemy_turn_start:
for slot in 0..state.enemies.len() {
    let key = format!("char_e2_{slot}");
    if let Some(rem) = state.stacks.get_mut(&key) {
        if *rem > 0.0 {
            *rem -= 1.0;
            if *rem <= 0.0 {
                if let Some(e) = state.enemies[slot].as_mut() { e.vulnerability -= 7.0; }
            }
        }
    }
}
```

### Energy Regen Rate Multiplier
```rust
let err_mult = 1.0 + state.team[idx].buffs.energy_regen_rate / 100.0;
state.team[idx].energy = (state.team[idx].energy + 10.0 * err_mult).min(max_energy);
```

### Debuff Count Check
```rust
let debuffs_on_target = target_idx
    .and_then(|t| state.enemies.get(t)).and_then(|s| s.as_ref())
    .map(|e| e.debuff_count).unwrap_or(0);
```
