# Aventurine Kit Definition

## Overview
- **Path**: Preservation
- **Element**: Imaginary
- **Role**: Tank / Sub-DPS
- **Core Mechanic**: **Fortified Wager** (Shield) and **Blind Bet** (Points). Aventurine provides stackable shields to all allies and accumulates Blind Bet points when shielded allies are attacked. At 7 points, he launches a powerful Follow-Up attack. All his abilities scale with DEF.

## Abilities
### Basic ATK : Straight Bet
Single Target

Deals Imaginary DMG equal to {100%} of Aventurine's DEF to a single target enemy.

Break: 10

### Skill : Cornerstone Deluxe
Defense

Provides all allies with a **Fortified Wager** shield that can block DMG equal to {24%} of Aventurine's DEF plus {320}, lasting for 3 turns. When gaining Fortified Wager again, the shield effect can stack, up to {200%} of the current shield effect.

### Ultimate : Roulette Shark
Single Target

Randomly gains 1 to 7 points of **Blind Bet**, then inflicts **Unnerved** on a single target enemy for 3 turns and deals Imaginary DMG equal to {270%} of Aventurine's DEF to the target. When an ally hits an Unnerved enemy, the CRIT DMG dealt increases by {15%}.

Cost: 110 Energy
Break: 30

### Talent : Shot Loaded Right
Passive

For any ally with **Fortified Wager**, their Effect RES increases by {50%}. When they are attacked, Aventurine gains 1 point of **Blind Bet**. When Aventurine has Fortified Wager, he can resist Crowd Control debuffs. This effect can be triggered again after 2 turns. Aventurine also gains 1 point of Blind Bet after being attacked.
Upon reaching 7 points of Blind Bet, Aventurine consumes 7 points to launch a **Follow-Up ATK** of 7 hits, with each hit dealing Imaginary DMG equal to {25%} of Aventurine's DEF to a single random enemy. Blind Bet is capped at 10 points.

Break: 3 per hit

## Major Traces
### A2: (Unnamed in code)
For every 100 of Aventurine's DEF that exceeds 1600, his own CRIT Rate increases by {2%}, up to a maximum increase of {48%}.

### A4: (Unnamed in code)
When battle starts, grants all allies a **Fortified Wager** shield, whose shield effect is equal to {100%} of the one granted by the Skill.

### A6: (Unnamed in code)
After Aventurine launches his Talent's Follow-Up ATK, provides all allies with a **Fortified Wager** shield that can block DMG equal to {7.2%} of Aventurine's DEF plus {96}, and additionally provides the ally with the lowest shield effect with a Fortified Wager shield that can block DMG equal to {7.2%} of Aventurine's DEF plus {96}, lasting for 3 turns.

## Minor Traces :
DEF +35%
Imaginary DMG +14.4%
Effect RES +10%

## Eidolons :

### E1
Increases CRIT DMG by {20%} for allies with Fortified Wager. After using the Ultimate, provides all allies with a Fortified Wager shield, whose shield effect is equal to {100%} of the one granted by the Skill.

### E2
When using Basic ATK, reduces the target's All-Type RES by {12%} for 3 turns.

### E3
Ultimate Lv. +2, up to a maximum of Lv. 15. Basic ATK Lv. +1, up to a maximum of Lv. 10.

### E4
When triggering his Talent's Follow-Up ATK, first increases Aventurine's DEF by {40%} for 2 turns and additionally increases the number of hits per Follow-Up ATK by 3 (from 7 to 10).

### E5
Skill Lv. +2, up to a maximum of Lv. 15. Talent Lv. +2, up to a maximum of Lv. 15.

### E6
For every ally that has a shield, the DMG dealt by Aventurine increases by {50%}, up to a maximum increase of {150%}.

## Gameplay Notes :

### DEF Scaling
All of Aventurine's damage and shield values scale strictly with his DEF. In the simulation, ATK% buffs are repurposed as DEF% buffs for him.

### Blind Bet Accumulation
- Shielded non-Aventurine ally hit: +1 BB.
- Shielded Aventurine hit: +2 BB total (+1 from "shielded ally" and +1 from "himself").
- Ultimate: +1 to +7 BB.

## Simulation Logic
- **onBattleStart**: Fold Lightcone DEF into base DEF. Apply A2 CRIT Rate boost. Apply A4 shields. Apply E1 global CRIT DMG boost.
- **onTurnStart**: Decrement E4 DEF boost counter.
- **onBeforeAction**: Apply E6 DMG boost based on shielded allies.
- **onAfterAction**: 
    - Handle Basic ATK: apply E2 RES debuff.
    - Handle Skill: apply shields to all allies (stackable and capped).
- **onUlt**: Gain random BB (1–7). Apply Unnerved debuff. Deal 270% DEF DMG. Trigger E1 shield gain.
- **onEnemyAction**: Accumulate BB points when shielded allies are attacked. Trigger FUP at 7+ points.
- **onEnemyTurnStart**: Handle Unnerved and E2 RES debuff duration tracking. Revert buffs/debuffs on expiry.
- **fireTalentFUP**: Consume 7 BB. Apply E4 DEF boost and hits. Deal 7 (or 10) random hits. Apply A6 mini-shields to all allies.