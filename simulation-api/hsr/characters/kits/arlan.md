# Arlan Kit Definition

## Overview
- **Path**: Destruction
- **Element**: Lightning
- **Role**: Main DPS
- **Core Mechanic**: **Pain and Anger** (Talent). Arlan's Skill consumes HP instead of Skill Points, and his damage increases as his current HP percentage decreases.

## Abilities
### Basic ATK : Lightning Rush
Single Target

Deals Lightning DMG equal to {100%} of Arlan's ATK to a single target enemy.

Break: 10

### Skill : Shackle Breaker
Single Target

Consumes HP equal to {15%} of Arlan's Max HP to deal Lightning DMG equal to {240%} of Arlan's ATK to a single target enemy. If Arlan's current HP is insufficient, his HP is reduced to 1 after using the Skill.

Break: 20

### Ultimate : Frenzied Punishment
Blast

Deals Lightning DMG equal to {320%} of Arlan's ATK to a single target enemy and Lightning DMG equal to {160%} of Arlan's ATK to enemies adjacent to it.

Break: 20 (main and adjacent)

### Talent : Pain and Anger
Passive

Increases Arlan's DMG for every {1%} of HP below his Max HP, up to a maximum of {72%} extra DMG.

## Major Traces
### A2: (Unnamed in code)
If Arlan's HP is {30%} or lower when defeating an enemy, immediately restores HP equal to {20%} of his Max HP.

### A4: (Unnamed in code)
The chance to resist DoT debuffs increases by {50%}.

### A6: (Unnamed in code)
When entering battle, if Arlan's HP is {50%} or lower, he nullifies all DMG received (excluding DoT) until he is hit.

## Minor Traces :
ATK +28%
HP +10%
Effect RES +18%

## Eidolons :

### E1
When HP is {50%} or lower, increases Skill DMG by {10%}.

### E2
Using Skill or Ultimate removes 1 debuff from Arlan.

### E3
Skill Lv. +2, up to a maximum of Lv. 15. Basic ATK Lv. +1, up to a maximum of Lv. 10.

### E4
When struck by a lethal blow, instead of becoming knocked out, Arlan immediately restores HP to {25%} of his Max HP. This effect is removed after it is triggered once or after 2 turns have elapsed.

### E5
Ultimate Lv. +2, up to a maximum of Lv. 15. Talent Lv. +2, up to a maximum of Lv. 15.

### E6
When HP is {50%} or lower, Ultimate deals {20%} more DMG. The DMG multiplier for adjacent targets is increased to be equal to the main target's DMG multiplier ({320%}).

## Gameplay Notes :

### HP Management
Arlan's Skill is the primary way to lower his HP to maximize his Talent's DMG boost. He does not consume Skill Points (SP).

### Survival
E4 provides a one-time safety net, and A6 provides a shield at the start of battle if Arlan is already at low HP. A2 provides some self-healing on kills.

## Simulation Logic
- **onBattleStart**: Apply A4 DoT resist. Apply A6 damage nullification if HP ≤ 50%. Wrap damage logic for E4 survival and A6 shield consumption.
- **onTurnStart**: Decrement E4 turn counter.
- **onBeforeAction**: 
    - Consume HP for Skill.
    - Calculate Talent DMG boost based on missing HP.
    - Apply E1 Skill boost and E6 Ultimate boost if HP ≤ 50%.
- **onAfterAction**: 
    - Trigger E2 debuff removal for Skill.
    - Trigger A2 HP restoration if an enemy was defeated and Arlan's HP ≤ 30%.
- **onUlt**: 
    - Trigger E2 debuff removal.
    - Calculate Blast damage (with E6 multiplier adjustment if HP ≤ 50%).
    - Trigger A2 HP restoration if an enemy was defeated and Arlan's HP ≤ 30%.