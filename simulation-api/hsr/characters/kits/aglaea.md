# Aglaea Kit Definition

## Overview
- **Path**: Remembrance
- **Element**: Lightning
- **Role**: Main DPS / Speed Buffer
- **Core Mechanic**: **Memosprite Garmentmaker** and **Supreme Stance**. Garmentmaker generates SPD Boost stacks. Ultimate (Supreme Stance) enhances Aglaea's Basic ATK to a Joint ATK with Garmentmaker and transfers SPD Boost to Aglaea.

## Abilities
### Basic ATK : Thorned Nectar
Single Target

Deals Lightning DMG equal to {100%} of Aglaea's ATK to a single target enemy.
In **Supreme Stance**, this is enhanced to a **Joint ATK**.

Break: 10

### Skill : Rise, Exalted Renown
Support

Summons **Garmentmaker** if it is not on the field. If Garmentmaker is already on the field, restores its HP by {50%} of its Max HP. Aglaea gains 20 Energy. This skill does not deal damage.

### Ultimate : Dance, Destined Weaveress
Enhance

Aglaea enters **Supreme Stance** and summons or restores **Garmentmaker** to Max HP.
In Supreme Stance:
- Aglaea's SPD is increased by {15%} for each SPD Boost stack.
- Aglaea's Basic ATK is enhanced to a **Joint ATK**.
- A countdown entity is created (SPD 100). When the countdown acts, Supreme Stance ends and Garmentmaker is dismissed.

Cost: 350 Energy

### Talent : Rosy-Fingered
Passive

When Aglaea or Garmentmaker attacks an enemy, applies **Seam Stitch** to the target. Only one enemy can have Seam Stitch at a time.
When an ally attacks an enemy with Seam Stitch, Aglaea deals Additional Lightning DMG equal to {30%} of her ATK and gains 10 Energy.
When hitting an enemy with Seam Stitch, Garmentmaker gains 1 stack of **SPD Boost** (max 6 stacks, or 7 at E4). Each stack increases Garmentmaker's SPD by 55.

## Major Traces
### A2: (Unnamed in code)
In Supreme Stance, Aglaea and Garmentmaker receive a flat ATK boost equal to 720% of Aglaea's SPD + 360% of Garmentmaker's SPD.

### A4: (Bloom of Drying Grass)
When Garmentmaker is dismissed, Aglaea retains up to 1 SPD Boost stack and gains 20 Energy.

### A6: (Unnamed in code)
When the battle starts, if Aglaea's Energy is less than 175, it is increased to 175.

## Minor Traces :
Crit Rate +12%
Lightning DMG +22.4% (Note: Implemented as flat stat boosts in simulation)

## Eidolons :

### E1
When attacking an enemy with Seam Stitch, Aglaea gains an additional 20 Energy, and the target takes 15% increased DMG.

### E2
When Aglaea or Garmentmaker takes action, they gain a stack that increases DEF Ignore by 14%, stacking up to 3 times (max 42%).

### E3
Skill Lv. +2, up to a maximum of Lv. 15. Basic ATK Lv. +1, up to a maximum of Lv. 10.

### E4
Max SPD Boost stacks increased to 7. When Aglaea attacks, Garmentmaker also gains 1 SPD Boost stack.

### E5
Ultimate Lv. +2, up to a maximum of Lv. 15. Talent Lv. +2, up to a maximum of Lv. 15.

### E6
In Supreme Stance, Aglaea and Garmentmaker gain 20% All-Type RES PEN. Additionally, Joint ATK DMG is increased based on Aglaea's SPD:
- > 160 SPD: +10% DMG
- > 240 SPD: +30% DMG
- > 320 SPD: +60% DMG

## Gameplay Notes :

### Garmentmaker
- Summoned with 66% + 720 of Aglaea's Max HP.
- Base SPD: 35% of Aglaea's base SPD + 55 per SPD Boost stack.
- Attacks (Thorned Snare) deal 110% ATK to main target and 66% to adjacent targets (Blast).
- Generates 10 Energy for Aglaea per attack.

### Joint ATK (Supreme Stance)
- Aglaea deals 200% ATK to main target and 90% to adjacent targets.
- Garmentmaker deals 200% ATK to main target and 90% to adjacent targets.
- Total Toughness Damage: 40 to main target, 20 to adjacent targets.

## Simulation Logic
- **onBattleStart**: Set Energy to 175 (A6). Initialize stacks.
- **onTurnStart**: Sync Aglaea's SPD with SPD Boost stacks if in Supreme Stance.
- **onBeforeAction**: Apply E2 DEF ignore, E6 RES PEN, E1 vulnerability, and A2 flat ATK boost. Suppress standard Basic ATK if in Supreme Stance.
- **onAfterAction**: 
    - Handle Skill (summon/heal Garmentmaker, 100% Action Advance).
    - Handle Joint ATK (damage calculation for Aglaea and Garmentmaker, energy gain, SPD stacks).
    - Handle standard Basic ATK (Seam Stitch application, energy gain).
- **onUlt**: Enter Supreme Stance, summon/restore Garmentmaker, sync SPD, and queue countdown.
- **onGlobalDebuff**: (Not applicable for Aglaea core mechanics).
- **onSeamStitchDmg**: Triggered when any ally hits the Seam Stitch target. Deals damage and grants energy to Aglaea.