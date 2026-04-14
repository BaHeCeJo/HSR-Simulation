# Argenti Kit Definition

## Overview
- **Path**: Erudition
- **Element**: Physical
- **Role**: Main DPS
- **Core Mechanic**: **Apotheosis** stacks and Dual-Phase Ultimate. Apotheosis stacks grant CRIT Rate (and CRIT DMG at E1). Argenti has two Ultimates sharing the same energy resource (max 180 Energy).

## Abilities
### Basic ATK : Fleeting Fragrance
Single Target

Deals Physical DMG equal to {100%} of Argenti's ATK to a single target enemy.

Break: 10

### Skill : Justice, Hereby Blooms
AoE

Deals Physical DMG equal to {120%} of Argenti's ATK to all enemies.

Break: 10

### Ultimate : Argenti Ultimate
The Ultimate has two phases based on Energy consumed:
1. **90 Energy**: *For In This Garden Supreme Beauty Bestows*. Deals Physical DMG equal to {160%} of Argenti's ATK to all enemies.
2. **180 Energy**: *Merit Bestowed in "My" Garden*. Deals Physical DMG equal to {280%} of Argenti's ATK to all enemies, and further deals DMG 6 extra times. Each time deals Physical DMG equal to {95%} of Argenti's ATK to a random enemy.

Break: 20 (all versions)

### Talent : Sublime Object
Passive

For every enemy hit by Argenti's Basic ATK, Skill, or Ultimate, Argenti gains 3 Energy and 1 stack of **Apotheosis**, up to 10 stacks (12 at E4). Each stack of Apotheosis increases Argenti's CRIT Rate by {2.5%}.

## Major Traces
### A2: (Unnamed in code)
At the start of his turn, Argenti immediately gains 1 stack of Apotheosis.

### A4: (Unnamed in code)
When enemies enter the battle, Argenti immediately gains 2 Energy for each enemy.

### A6: (Unnamed in code)
Deals 15% more DMG to enemies whose HP is 50% or less.

## Minor Traces :
ATK +28%
Physical DMG +14.4%

## Eidolons :

### E1
Each stack of Apotheosis additionally increases CRIT DMG by 4%.

### E2
When using the Ultimate, if there are 3 or more enemies on the field, ATK increases by 40% for 1 turn.

### E3
Skill Lv. +2, up to a maximum of Lv. 15. Talent Lv. +2, up to a maximum of Lv. 15.

### E4
At the start of battle, gains 2 stacks of Apotheosis. The maximum number of Apotheosis stacks increases to 12.

### E5
Ultimate Lv. +2, up to a maximum of Lv. 15. Basic ATK Lv. +1, up to a maximum of Lv. 10.

### E6
When using Ultimate, ignores 30% of the enemies' DEF.

## Gameplay Notes :

### Ultimate Selection
In the simulation, Argenti prefers to wait for 180 Energy to fire his Ultimate. This can be overridden by setting `state.stacks['argenti_prefer_90_ult'] = 1`.

### Energy Generation
Argenti's talent grants 3 Energy per enemy hit. 
- Basic ATK (1 enemy hit): 20 base + 3 talent = 23 Energy.
- Skill (e.g., 5 enemies hit): 30 base + 15 talent = 45 Energy.
- Ultimate: 5 base + (enemies hit * 3) talent.

## Simulation Logic
- **onBattleStart**: Initialize Energy and Apotheosis. Apply A4 energy and E4 stacks.
- **onTurnStart**: Apply A2 Apotheosis stack.
- **onBeforeAction**: Apply Apotheosis CRIT buffs (E1 included). Apply E2 ATK boost and E6 DEF ignore for Ultimate.
- **onAfterAction**: Handle energy and Apotheosis gain based on enemies hit for Basic and Skill.
- **onUlt**: Determine which version of the Ultimate to use (90 vs 180). Calculate damage (including random hits for the 180 version). Apply A6 DMG boost. Handle talent procs (energy and stacks) for all hits landed during the Ultimate.