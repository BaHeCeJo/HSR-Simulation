# Pela Kit Definition

## Overview
- **Path**: Nihility
- **Element**: Ice
- **Role**: Support / Debuffer
- **Core Mechanic**: AOE DEF Reduction via Ultimate. Energy refund via Talent when attacking debuffed enemies.

## Abilities

### Basic: Frost Shot
- **Multiplier**: 100% ATK (Lv. 6)

### Skill: Frostbite
- **Multiplier**: 210% ATK (Lv. 10)
- **Effect**: Removes 1 buff from a single enemy. Considered a debuff for certain mechanics.

### Ultimate: Zone Suppression
- **Multiplier**: 100% ATK (Lv. 10)
- **Effect**: 100% base chance to reduce DEF of all enemies by 40% (Lv. 10) for 2 turns.

### Talent: Data Collecting
- **Effect**: If the enemy is debuffed after Pela's attack, she gains 10 extra Energy.

## Eidolons
- **E4**: Skill has a 100% base chance to reduce the target's Ice RES by 12% for 2 turns.
- **E6**: When attacking a debuffed enemy, Pela deals additional Ice DMG equal to 40% of her ATK.

## Simulation Logic
- **onBeforeAction**: 
    - Ultimate: Mark as `inflictsDebuff`, apply 40% DEF reduction.
    - Skill: Mark as `inflictsDebuff`.
- **onAfterAction**: (To be implemented) Handle energy refund from Talent if target is debuffed.
