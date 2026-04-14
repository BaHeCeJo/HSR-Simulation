# Ashveil Kit Definition

## Overview
- **Path**: Hunt
- **Element**: Lightning
- **Role**: Support / Follow-Up DPS
- **Core Mechanic**: **Bait** and **Gluttony**. Ashveil marks an enemy with Bait, which provides global DEF Ignore and triggers Ashveil's Follow-Up attacks when allies attack. Gluttony stacks amplify her Follow-Up DMG and enable a powerful chain during her Ultimate.

## Abilities
### Basic ATK : Talons: Inculcate Decorum
Single Target

Deals Lightning DMG equal to {100%} of Ashveil's ATK to a single target enemy.

Break: 10

### Skill : Flog: Smite Evil
Single Target

Deals Lightning DMG equal to {200%} of Ashveil's ATK to a single target enemy and applies **Bait**. If the target already has Bait, the DMG multiplier is increased to {300%}, and Ashveil restores 1 Skill Point.
Gains 1 stack of **Gluttony**.

Break: 20

### Ultimate : Banquet: Insatiable Appetite
Single Target

Deals Lightning DMG equal to {400%} of Ashveil's ATK to a single target enemy and applies **Bait**.
Restores Ashveil's **Charge** to 3 and gains 2 stacks of **Gluttony**.
Triggers an **Enhanced Follow-Up ATK chain**:
- Performs a Follow-Up ATK (200% ATK) on the Bait target.
- While Ashveil has 4 or more Gluttony stacks, consumes 4 stacks to perform another Follow-Up ATK. This repeats until Gluttony is less than 4 or all enemies are defeated.

Break: 30

### Talent : Rancor: Enmity Reprisal
Passive

When an ally attacks the enemy marked with **Bait**, Ashveil launches a **Follow-Up ATK** dealing Lightning DMG equal to {200%} of her ATK to the target. This consumes 1 **Charge**.
Ashveil starts the battle with 2 Charges (max 3). Each Follow-Up ATK grants 2 stacks of **Gluttony** and 8 Energy.

## Major Traces
### A2: (Unnamed in code)
- Skill grants +1 Gluttony.
- Ultimate grants +2 Gluttony.
- If an enemy is defeated during a Follow-Up ATK, Ashveil gains 1 additional Gluttony stack and moves Bait to the alive enemy with the lowest HP.

### A4: (Unnamed in code)
Follow-Up ATK DMG is increased by {80%}. Additionally, each stack of Gluttony increases Follow-Up ATK DMG by {10%}.

### A6: (Unnamed in code)
While Ashveil is on the field, all allies' CRIT DMG is increased by {40%}. Ashveil's Follow-Up attacks receive an additional {80%} CRIT DMG.

## Minor Traces :
ATK +10%
Lightning DMG +14.4%
CRIT DMG +37.3%

## Eidolons :

### E1
All enemies take 24% increased DMG. If an enemy's HP is 50% or less, they take 36% increased DMG instead.

### E2
The maximum number of Gluttony stacks is increased to 18. When the Enhanced Follow-Up chain triggers during the Ultimate, Ashveil restores Gluttony stacks equal to 35% of the total stacks consumed during the chain.

### E3
Ultimate Lv. +2, up to a maximum of Lv. 15. Basic ATK Lv. +1, up to a maximum of Lv. 10.

### E4
After using her Ultimate, Ashveil's ATK is increased by 40% for 3 turns.

### E5
Skill Lv. +2, up to a maximum of Lv. 15. Talent Lv. +2, up to a maximum of Lv. 15.

### E6
While **Bait** is active on the field, all enemies' All-Type RES is reduced by 20%. Additionally, Ashveil's DMG is increased by 4% for every Gluttony stack she has ever gained in the current battle, up to a maximum of 30 stacks (120% DMG boost).

## Gameplay Notes :

### Bait Mechanic
- Only one enemy can be marked with Bait at a time.
- While an enemy is marked with Bait, all allies gain +40 DEF Ignore.
- If the current Bait target is defeated, Bait automatically moves to the alive enemy with the lowest HP.

### Gluttony
- Max stacks: 12 (18 at E2).
- Used as a resource for the Ultimate's Follow-Up chain.
- Provides significant DMG boosts to Follow-Up attacks.

## Simulation Logic
- **onBattleStart**: Initialize Charges (2) and Gluttony (0). Apply A6 CRIT DMG boost and E1 vulnerability.
- **onTurnStart**: Decrement E4 ATK buff. Refresh E1 vulnerability based on enemy HP.
- **onBeforeAction**: Apply E4 ATK and E6 DMG boosts. Check for Skill SP refund condition.
- **onAfterAction**: Handle Skill Bait application, SP refund, and Gluttony gain.
- **onUlt**: Apply Bait, E4 buff, and deal 400% DMG. Restore Charges and Gluttony. Launch the Enhanced Follow-Up chain (looping consumption of Gluttony).
- **onAllyAction**: Trigger Talent Follow-Up if an ally hits the Bait target and Ashveil has Charges. Gain Gluttony and Energy. Handle Bait movement on kill.