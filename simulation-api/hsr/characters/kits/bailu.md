# Bailu Kit Definition

## Overview
- **Path**: Abundance
- **Element**: Lightning
- **Role**: Sustain / Healer
- **Core Mechanic**: **Invigoration** and **Gourdful of Elixir** (Talent). Bailu provides multi-target healing and a unique "Invigoration" buff that heals allies when they are attacked. She also has a one-time (twice at E6) life-saving revive. All her healing scales with her Max HP.

## Abilities
### Basic ATK : Diagnostic Kick
Single Target

Deals Lightning DMG equal to {100%} of Bailu's ATK to a single target enemy.

Break: 10

### Skill : Singing Among Clouds
Restore

Heals a single ally for {11.7%} of Bailu's Max HP plus {312}. Bailu then heals random allies 2 extra times. Each extra heal's amount is reduced by {15%} compared to the previous heal.

### Ultimate : Felicitous Thunderleap
Restore

Heals all allies for {13.5%} of Bailu's Max HP plus {360}.
Bailu applies **Invigoration** to allies that are not already Invigorated. For those who are already Invigorated, she extends their Invigoration duration by 1 turn.
Invigoration lasts for 2 turns. This effect cannot stack.

Cost: 100 Energy

### Talent : Gourdful of Elixir
Passive

When an ally with **Invigoration** is hit, restores the ally's HP for {5.4%} of Bailu's Max HP plus {144}. This effect can be triggered 2 times (3 times with A4).
When an ally receives a killing blow, they will not be knocked out. Bailu immediately heals the ally for {18%} of her Max HP plus {480}. This effect can be triggered 1 time per battle (2 times at E6).

## Major Traces
### A2: (Unnamed in code)
Bailu's Max HP increases by {10%}.

### A4: (Unnamed in code)
Invigoration's heal trigger count increases by 1 (from 2 to 3).

### A6: (Unnamed in code)
Invigorated allies take {10%} less DMG.

## Minor Traces :
HP +28%
DEF +22.5%
Effect RES +10%

## Eidolons :

### E1
If the ally's current HP is equal to their Max HP when Invigoration ends, Bailu restores {8} Energy to the ally.

### E2
After using her Ultimate, Bailu's Outgoing Healing increases by {15%} for 2 turns.

### E3
Skill Lv. +2, up to a maximum of Lv. 15. Talent Lv. +2, up to a maximum of Lv. 15.

### E4
Every heal provided by the Skill increases the recipient's DMG dealt by {10%}, stacking up to 3 times for 2 turns.

### E5
Ultimate Lv. +2, up to a maximum of Lv. 15. Basic ATK Lv. +1, up to a maximum of Lv. 10.

### E6
Increases the number of times Bailu can prevent an ally from being knocked out during a single battle by 1.

## Gameplay Notes :

### Invigoration
- Applied by Ultimate and Technique.
- Heals the ally when they take damage.
- Provides 10% DMG reduction (A6).
- Trigger count is shared across the team and resets when Bailu uses her Ultimate.

### KO Prevention (Revive)
- Triggers automatically when an ally would be defeated.
- The ally is healed and remains on the field.
- Can only happen once per battle (twice at E6).

## Simulation Logic
- **onBattleStart**: Apply Invigoration to all allies (Technique). Initialize revive and talent trigger counters. Wrap damage logic for A6 DMG reduction and Talent KO prevention.
- **onTurnStart**: Decrement E2 Outgoing Healing boost.
- **onAfterAction**: Handle Skill 3-hit cascade heal. Apply E4 DMG boost to each healed target.
- **onUlt**: Heal all allies. Apply or extend Invigoration. Reset talent trigger counter to 3. Apply E2 boost.
- **onEnemyAction**: Trigger Talent Invigoration heal for the lowest-HP invigorated ally if triggers are available.