# Honkai: Star Rail Optimizer (Plugin Specification)

This document outlines the architecture and implementation plan for the HSR-specific optimization module within GachaStats.

## 1. Core Objectives
The optimizer provides data-driven recommendations for the "Best in Slot" (BiS) configurations based on a user's **actual owned inventory** and specific endgame combat scenarios.

- **Character Optimization:** Calculate the best Relic and Light Cone combinations for a specific character.
- **Team Synergy Engine:** Identify the best teammates and team compositions for a given character or fight.
- **Endgame Simulation:** Given a specific fight (e.g., Memory of Chaos, Pure Fiction), determine the optimal team from the user's collection to achieve the fastest clear/highest score.

## 2. Core Mechanics (HSR Specific)

### 2.1. Character Stats
To simulate combat, the engine must account for all primary and secondary stats:
- **HP:** How much damage a Character can take before falling in combat.
- **ATK:** Determines the base damage of a Character's attacks.
- **DEF:** Reduces the damage a Character takes from enemy hits.
- **Speed (SPD):** Determines how fast and how often a Character acts. Essential for "Action Value" calculations.
- **Crit Rate:** Likelihood of landing a critical hit.
- **Crit DMG:** The damage multiplier applied upon a critical hit.
- **Break Effect:** Enhances Weakness Break damage, increases DoT damage, and determines how far enemy actions are delayed.
- **Outgoing Healing Boost:** Increases the effectiveness of a Character’s healing abilities.
- **Energy Regeneration Rate (ERR):** Controls how quickly the Ultimate ability is charged.
- **Effect Hit Rate (EHR):** Likelihood of applying debuffs to enemies.
- **Effect RES:** Resistance against enemy-applied debuffs.
- **Elemental DMG Boost:** Increases damage of specific types (Physical, Fire, Ice, Lightning, Wind, Quantum, Imaginary).

### 2.2. Paths (Roles)
Paths function as character classes and dictate utility:
- **Destruction:** Focus on Blast and Single-target damage; balances survivability with offensive power.
- **Hunt:** Specialized in high Single-target damage for boss encounters.
- **Erudition:** Specialists in multi-target/AoE damage for clearing mobs.
- **Harmony:** Buffers that focus on supporting allies to increase team damage.
- **Nihility:** Focus on debuffs and Damage over Time (DoT) to weaken enemies.
- **Preservation:** Defensive specialists focused on team survival via shields and mitigation.
- **Abundance:** Pure healers dedicated to keeping the team alive.
- **Remembrance:** Focused on utilizing a summoned **Memosprite** unit to assist in battle.
- **Elation:** Focused on building unique resource bars to unleash powerful specialized abilities.

### 2.3. Ability Structure (Skill Logic)
Each Character possesses 5 unique abilities that define their action loop:
- **Basic Attack:** Filler damage and the primary source of **Skill Point (SP)** generation.
- **Skill:** Tactical combat ability. Has no cooldown but consumes SP (generally).
- **Ultimate:** The strongest ability. Can be cast at any time (even during enemy turns) once Energy is full.
- **Talent:** Passive ability providing conditional benefits (e.g., "Follow-up attacks when an ally is hit").
- **Technique:** Overworld ability used **before combat begins**. Can either initiate combat with a special effect or provide a buff prior to engagement.

## 3. Technical Components

### 3.1. Relic Inventory System (`user_relics`)
A specialized storage layer for user-owned gear.
- **Fields:** Slot (Head, Hands, Body, Feet, Sphere, Rope), Set, Main Stat, Sub-stats, Rarity, Level.
- **Sub-stat Logic:** Average roll values (e.g., SPD 2.3/2.6/2.9) used for precise breakpoint calculations.

### 3.2. Combat Data Model
To perform calculations, we need to map HSR's math:
- **Character Stats:** Base stats at Level 80.
- **Ability Multipliers:** Mapping `entity_abilities` to raw numeric formulas.
- **Action Gauge:** Simulating the 10,000 Action Value (AV) system.

### 3.3. Optimization Engine
A TypeScript-based simulation worker that runs:
- **Permutation Search:** Iterates through relic combinations to maximize "Combat Score" for specific fights.
- **Synergy Scoring:** Weights teammates based on Path/Element compatibility and buff overlap.

## 4. Implementation Roadmap

### Phase 1: Foundations (The "Inventory")
- Implement the `user_relics` table and a management UI in the User Profile.
- Build a basic "Stat Calculator" showing total stats with selected relics.

### Phase 2: The Logic (The "Solver")
- Create the calculation engine for Damage and Survivability.
- Add "Target Stat" optimization (e.g., "Find build with 134 SPD and max Crit").

### Phase 3: The Simulator (The "Strategist")
- Implement the Enemy/Fight database.
- Build the "Team Optimizer" for Memory of Chaos / Pure Fiction / Apocalyptic Shadow.
