# Asta Kit Definition

## Overview
- **Path**: Harmony
- **Element**: Fire
- **Role**: Support
- **Core Mechanic**: **Charging** stacks (max 5). Gained by hitting unique enemies (bonus if Fire Weak). Each stack provides a global ATK boost to all allies. Her Ultimate provides a massive SPD boost to the entire team.

## Abilities
### Basic ATK : Spectrum Beam
Single Target

Deals Fire DMG equal to {100%} of Asta's ATK to a single target enemy.

Break: 10

### Skill : Meteor Storm
Bounce

Deals Fire DMG equal to {50%} of Asta's ATK to a single target enemy and further deals DMG 4 extra times. Each extra hit deals Fire DMG equal to {50%} of Asta's ATK to a random enemy.
(At E1, fires 1 extra bounce for a total of 6 hits).

Break: 10 per hit

### Ultimate : Astral Blessing
Support

Increases the SPD of all allies by {50} for 2 turns.

Cost: 120 Energy

### Talent : Astrometry
Passive

Gains 1 stack of **Charging** for every different enemy hit by Asta, and an extra 1 stack if the enemy hit has Fire Weakness.
For every Charging stack Asta has, all allies' ATK increases by {14%}, up to 5 times.
Starting from Asta's second turn, the number of Charging stacks she has will be reduced by 3 at the beginning of every turn (2 at E6).

## Major Traces
### A2: (Unnamed in code)
Asta's Basic ATK has an {80%} base chance to Burn the target for 3 turns. Burned enemies take Fire DoT equal to {50%} of the Basic ATK's DMG.

### A4: (Unnamed in code)
When Asta is on the field, all Fire-element allies' Fire DMG increases by {18%}.

### A6: (Unnamed in code)
Asta's DEF increases by {6%} for every Charging stack she has.

## Minor Traces :
Fire DMG +22.4%
CRIT Rate +6.7%
DEF +22.5%

## Eidolons :

### E1
When using Skill, deals DMG for 1 extra time to a random enemy.

### E2
After using her Ultimate, Asta's Charging stacks will not be reduced in the next turn.

### E3
Skill Lv. +2, up to a maximum of Lv. 15. Talent Lv. +2, up to a maximum of Lv. 15.

### E4
Asta's Energy Regeneration Rate increases by {15%} when she has 2 or more Charging stacks.

### E5
Ultimate Lv. +2, up to a maximum of Lv. 15. Basic ATK Lv. +1, up to a maximum of Lv. 10.

### E6
Charging stack reduction per turn is decreased by 1 (reduces by 2 instead of 3).

## Gameplay Notes :

### Charging Stacks
- Asta gains stacks per *unique* enemy hit in an action.
- Skill is the most efficient way to gain stacks against multiple enemies.
- E2 helps maintain stacks after an Ultimate.

### SPD Buff
- The SPD boost from her Ultimate lasts for 2 of each ally's turns.
- In the simulation, this is tracked per ally and expires after they take 2 actions.

## Simulation Logic
- **onBattleStart**: Initialize Charging stacks. Apply A4 Fire DMG boost to Fire allies.
- **onTurnStart**: Handle Charging stack reduction (skipping if E2 is active). Apply E6 reduction adjustment.
- **onBeforeAction**: Apply E4 Energy Regeneration Rate boost if Asta has 2+ stacks.
- **onAfterAction**: 
    - Handle Basic ATK: gain Charging stacks (bonus for Fire Weakness). Apply A2 Burn.
    - Handle Skill: perform bounces (E1 included). Gain Charging stacks per unique enemy hit (bonus for Fire Weakness).
- **onUlt**: Apply SPD boost to all allies. Set E2 skip flag.
- **onEnemyTurnStart**: Trigger A2 Burn DoT.
- **onAllyAction**: Decrement SPD buff duration for the acting ally.