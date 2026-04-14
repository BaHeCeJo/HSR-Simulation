# Honkai: Star Rail Game Mechanics & Simulator Guidelines

This document serves as the definitive reference for implementing HSR character kits and understanding the core mechanics of the combat simulator.

## 1. Timeline & Action Value (AV)

The HSR combat system is turn-based but governed by a continuous timeline.

- **Action Value (AV):** The "distance" on the timeline an actor must travel to reach their next turn.
- **Formula:** `AV = 10000 / Speed`
- **Current AV:** The total elapsed time since the battle started.
- **Max AV (Cycles):**
    - Cycle 0: 150 AV
    - Cycles 1+: 100 AV each
    - Total AV for $N$ cycles: $150 + (N-1) \times 100$

### Simulation Rules:
1. Actors are sorted in an `avQueue` by their `nextAV`.
2. The actor with the lowest `nextAV` takes the turn.
3. After a turn, `nextAV` is incremented by `10000 / CurrentSpeed`.
4. Speed buffs/debuffs instantly recalculate the *remaining* AV for the current action.

## 2. Damage Formula

The simulator uses the standard HSR damage formula:

`DMG = Base DMG * Multiplier * DMG Boost * DEF Mult * RES Mult * Vulnerability * Broken Mult * DMG Reduction`

- **Base DMG:** Scaling Stat (e.g., ATK) * Ability Multiplier.
- **Multiplier:** Special multiplicative buffs (e.g., Acheron's Nihility Trace).
- **DMG Boost:** `1 + (Elemental DMG% + All DMG% + specific DMG%)`.
- **DEF Mult:** `(Level + 20) / ((EnemyLevel + 20) * (1 - DEF_Ignore - DEF_Reduction) + (Level + 20))`.
- **RES Mult:** `1 - (BaseRES - RES_Pen)`. Base RES is usually 20% (0.2).
- **Vulnerability:** `1 + Received_DMG_Increase`.
- **Broken Mult:** 0.9 if enemy has toughness, 1.0 if broken.
- **DMG Reduction:** `1 - Reduction_Percentage`.

## 3. Character Kit Structure

Every character implementation in `lib/hsr/characters/` must follow this structure:

```typescript
export const CharacterName: CharacterKit = {
  id: "UUID",
  name: "Name",
  path: "Path",
  element: "Element",
  // Metadata for DB lookup
  slot_names: { basic: "...", skill: "...", ultimate: "...", talent: "..." },
  // Default scaling values (used if DB lookup fails)
  abilities: { ... },
  // Lifecycle hooks for the simulator
  hooks: {
    onBattleStart: (state, member) => { ... },
    onTurnStart: (state, member) => { ... },
    onBeforeAction: (state, member, action) => { ... }, // Buffs applied here
    onAfterAction: (state, member, action) => { ... },
    onUlt: (state, member) => { ... }, // Custom Ult logic
    onGlobalDebuff: (state, source, target) => { ... } // React to anyone landing a debuff
  },
  // Static modifiers
  special_modifiers: {
    energy_type: "ENERGY" | "STACKS",
    energy_cost: number,
    stat_boosts: (member) => { ... }, // Trace stat nodes
    eidolon_level_boosts: (eidolon) => { ... } // Which abilities get +2 levels
  }
};
```

## 4. Character-Specific Recaps

Each character file (`.ts`) must contain a comment block at the top summarizing the character's kit for quick AI context.

### Required Recap Fields:
- **Role:** Main DPS, Sub DPS, Support, Sustain.
- **Core Mechanic:** What makes the character unique (e.g., Acheron's Stacks, Blade's HP loss).
- **Skill Priority:** Which abilities to prioritize in the simulator.
- **Team Synergies:** Preferred paths or elements.
- **Eidolon Milestones:** Key eidolons that change behavior (e.g., Acheron E2).

---

## 5. Standard UUIDs (Commonly Used)
- **ATK:** `c987f652-6a0b-487f-9e4b-af2c9b51c6aa`
- **SPD:** `3e4b082d-7943-440d-ae2c-8d31b0a370be`
- **Crit Rate:** `a62e3a38-743a-41f8-8523-aec4ef998c84`
- **Crit DMG:** `a93e523a-7852-4580-b2ef-03467e214bcd`
- **Lightning RES:** `3de09fc5-7cb1-412f-aac0-0ebb0ba905e8`
