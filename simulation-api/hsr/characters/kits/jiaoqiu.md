# Jiaoqiu Kit Definition

## Overview
- **Path**: Nihility
- **Element**: Fire
- **Role**: Support / Debuffer / DoT
- **Core Mechanic**: **Ashen Roast** stacks (max 5). 
    - Increases DMG received (Vulnerability): 15% (1 stack) + 5% per extra stack (up to 35% at 5 stacks).
    - Counts as **Burn** (Fire DoT): 180% ATK per turn.
    - Stacks last 2 turns.

## Abilities

### Basic: Heart Afire
- **Multiplier**: 100% ATK (Lv. 6)
- **Effect**: 100% base chance to inflict 1 stack of Ashen Roast (via Talent).

### Skill: Scorch Onslaught
- **Main Target**: 150% ATK (Lv. 10)
- **Adjacent Targets**: 90% ATK (Lv. 10)
- **Effect**: 100% base chance to inflict 1 stack of Ashen Roast on the primary target. (Talent applies to all hit).

### Ultimate: Pyrograph Arcanum
- **Cost**: 100 Energy
- **Multiplier**: 100% ATK (Lv. 10)
- **Effect**: 
    - Sets all enemies' Ashen Roast stacks to the highest count on the field.
    - Activates a **Zone** for 3 turns.
    - **Zone Effect**: 
        - Enemies receive 15% increased Ultimate DMG.
        - 60% base chance to inflict 1 stack of Ashen Roast when an enemy takes action (max 6 triggers per Ult cast, once per enemy turn).
- **A6 Trace**: New enemies entering during Zone are inflicted with stacks equal to the highest on field (min 1).

### Talent: Quartet Finesse, Octave Finery
- **Effect**: Hitting an enemy with Basic, Skill, or Ult has a 100% base chance to inflict 1 stack of Ashen Roast.
- **Ashen Roast Scaling**: 15% + (stacks-1)*5% Vulnerability.
- **Burn DoT**: 180% ATK.

### Traces
- **A2**: Start battle with +15 Energy.
- **A4**: Every 15% EHR above 80% grants +60% ATK (Max +240% ATK at 140% EHR).
- **A6**: Zone applies stacks to entering enemies.

## Eidolons
- **E1**: Allies deal 40% increased DMG to enemies with Ashen Roast. Talent applies +1 extra stack (total 2 per trigger).
- **E2**: Ashen Roast Burn DoT multiplier +300% (Total 480% ATK).
- **E4**: While Zone exists, enemies ATK -15%.
- **E6**: Ashen Roast Max Stacks increased to 9. Each stack reduces All-Type RES by 3% (Max 27%). Stacks transfer on enemy death.

## Simulation Logic
- **onBattleStart**: Gain 15 Energy (A2).
- **onBeforeAction**:
    - If Skill/Ult: Mark `inflictsDebuff`.
    - Apply ATK boost based on EHR (A4).
- **onAfterAction**:
    - If hit enemies: Apply Ashen Roast stack(s) (1 or 2 if E1).
- **onEnemyTurnStart**:
    - If Zone is active: 60% base chance to apply Ashen Roast stack (respect trigger limit).
    - Apply Burn DoT DMG.
- **onUlt**:
    - Equalize stacks to max.
    - Set Zone duration to 3.
    - Reset Zone trigger count.
- **Vulnerability Calculation**: 
    - 15 + (stacks - 1) * 5 (Base)
    - + 15 if Ultimate DMG (Zone)
    - + 40 (E1)
- **RES PEN (E6)**: stacks * 3.
