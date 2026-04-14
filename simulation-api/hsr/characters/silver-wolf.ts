/**
 * @character Silver Wolf
 * @role Debuffer / Sub DPS
 * @core_mechanic Weakness implantation, DEF Shred, and "Bugs" (ATK/DEF/SPD reduction).
 * @skill_priority Ultimate > Skill > Basic
 * @team_synergies Acheron (Fast stacks), Quantum teams.
 * @eidolon_milestones E1 (Energy refund), E2 (Vulnerability + Bugs on ally hit), E4/E6 (Massive DMG boosts).
 */

import type { CharacterKit, Action, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

// UUIDs from HSR_ID_MAPPING.md
const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const EHR_ID = '764bb8fd-ad42-4bd9-8332-631187154d77';

export const SilverWolf: CharacterKit = {
  id: "2f2432f3-4736-4210-870a-c48d0c6bc3ee",
  slot_names: {
    basic: "System Warning",
    skill: "Allow Changes?",
    ultimate: "User Banned",
    talent: "Awaiting System Response...",
  },
  abilities: {
    basic: {
      ability_id: "7f8a1b2c-3d4e-5f6a-7b8c-9d0e1f2a3b4c",
      attribute_index: 0,
      default_multiplier: 1.0, // {100%}
      stat_id: ATK_ID
    },
    skill: {
      base_chance: {
        ability_id: "a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6",
        attribute_index: 0, // {120%}
        default_multiplier: 1.20,
        stat_id: EHR_ID
      },
      weakness_res: {
        ability_id: "a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6",
        default_multiplier: 0.20, // 20% (No index in MD)
        stat_id: "RES_REDUC_ID"
      },
      all_res: {
        ability_id: "a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6",
        attribute_index: 1, // {13%}
        default_multiplier: 0.13,
        stat_id: "RES_REDUC_ID"
      },
      main: {
        ability_id: "a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6",
        attribute_index: 2, // {196%}
        default_multiplier: 1.96,
        stat_id: ATK_ID
      }
    },
    ultimate: {
      base_chance: {
        ability_id: "b2c3d4e5-f6a7-b8c9-d0e1-f2a3b4c5d6e7",
        attribute_index: 0, // {120%}
        default_multiplier: 1.20,
        stat_id: EHR_ID
      },
      def_shred: {
        ability_id: "b2c3d4e5-f6a7-b8c9-d0e1-f2a3b4c5d6e7",
        attribute_index: 1, // {45%}
        default_multiplier: 0.45,
        stat_id: "DEF_SHRED_ID"
      },
      main: {
        ability_id: "b2c3d4e5-f6a7-b8c9-d0e1-f2a3b4c5d6e7",
        attribute_index: 2, // {380%}
        default_multiplier: 3.8,
        stat_id: ATK_ID
      }
    },
    talent: {
      bug_atk: {
        ability_id: "c3d4e5f6-a7b8-c9d0-e1f2-a3b4c5d6e7f8",
        attribute_index: 0, // {10%}
        default_multiplier: 0.10,
        stat_id: "ATK_REDUC_ID"
      },
      bug_def: {
        ability_id: "c3d4e5f6-a7b8-c9d0-e1f2-a3b4c5d6e7f8",
        attribute_index: 1, // {12%}
        default_multiplier: 0.12,
        stat_id: "DEF_REDUC_ID"
      },
      bug_spd: {
        ability_id: "c3d4e5f6-a7b8-c9d0-e1f2-a3b4c5d6e7f8",
        attribute_index: 2, // {6%}
        default_multiplier: 0.06,
        stat_id: "SPD_REDUC_ID"
      },
      base_chance: {
        ability_id: "c3d4e5f6-a7b8-c9d0-e1f2-a3b4c5d6e7f8",
        attribute_index: 3, // {100%}
        default_multiplier: 1.00,
        stat_id: EHR_ID
      }
    }
  },
  hooks: {
    onBattleStart: (state, member) => {
      // A4: Immediately regenerates 20 Energy
      state.stacks[member.characterId] = (state.stacks[member.characterId] || 0) + 20;
      
      // E2: When enemy enters battle, increases DMG received by 20%
      if (member.eidolon >= 2) {
          state.enemy.vulnerability += 20;
      }
    },
    onTurnStart: (state, member) => {
      // A4: Regenerates 5 Energy at start of her turn
      state.stacks[member.characterId] = (state.stacks[member.characterId] || 0) + 5;
    },
    onBeforeAction: (state, member, action) => {
      // A6: For every 10% EHR, +10% ATK (max 50%)
      const ehr = member.base_stats[EHR_ID] || 0; 
      member.buffs.atk_percent += Math.min(Math.floor(ehr / 10) * 10, 50);

      // E6: DMG Boost per debuff (20% each, max 100%)
      if (member.eidolon >= 6) {
          const debuffs = state.enemy.debuffCount || 0;
          member.buffs.dmg_boost += Math.min(debuffs * 20, 100);
      }

      if (action.type === 'skill') {
          action.inflictsDebuff = true;
          // Skill: 20% RES reduction (weakness) + 13% All-Type RES reduction
          state.enemy.activeDebuffs["sw_weakness_res"] = { duration: 3, value: 20, stat: "Weakness RES" };
          state.enemy.activeDebuffs["sw_all_res"] = { duration: 2, value: 13, stat: "All RES" };
      }

      if (action.type === 'ultimate') {
          action.inflictsDebuff = true;
          // Ultimate: 45% DEF reduction
          state.enemy.activeDebuffs["sw_ult_def"] = { duration: 3, value: 45, stat: "DEF reduction" };
          
          // E4: Additional DMG per debuff (20% ATK each, max 5)
          if (member.eidolon >= 4) {
              const debuffs = Math.min(state.enemy.debuffCount || 0, 5);
              action.multiplier += debuffs * 0.20;
          }
      }

      state.enemy.debuffCount = Object.keys(state.enemy.activeDebuffs).length;
    },
    onAfterAction: (state, member, action) => {
      // Talent: Apply Bug after attack (100% base chance)
      if (action.type === 'basic' || action.type === 'skill' || action.type === 'ultimate') {
          const bugs = ["sw_bug_atk", "sw_bug_def", "sw_bug_spd"];
          const randomBug = bugs[Math.floor(Math.random() * bugs.length)];
          const duration = 3 + (1); // A2 extends duration by 1
          
          state.enemy.activeDebuffs[randomBug] = { duration, value: 10 }; // Values scale per skill level usually
          state.enemy.debuffCount = Object.keys(state.enemy.activeDebuffs).length;
      }

      // E1: Energy refund after Ultimate
      if (action.type === 'ultimate' && member.eidolon >= 1) {
          const debuffs = Math.min(state.enemy.debuffCount || 0, 5);
          state.stacks[member.characterId] += debuffs * 7;
      }
    },
    onGlobalDebuff: (state, source, target) => {
        // E2: When enemy receives attack from ally, SW has 100% chance to implant Bug
        // Note: This logic might need careful placement to avoid loops, but following MD:
        const swMember = state.team.find(m => m.characterId === "2f2432f3-4736-4210-870a-c48d0c6bc3ee");
        if (swMember && swMember.eidolon >= 2) {
             const bugs = ["sw_bug_atk", "sw_bug_def", "sw_bug_spd"];
             const randomBug = bugs[Math.floor(Math.random() * bugs.length)];
             state.enemy.activeDebuffs[randomBug] = { duration: 3, value: 10 };
             state.enemy.debuffCount = Object.keys(state.enemy.activeDebuffs).length;
        }
    }
  },
  special_modifiers: {
    energy_type: "ENERGY",
    energy_cost: 110,
    stat_boosts: (state) => ({
      atk_percent: 28,
      dmg_boost: 8,
      ehr: 18
    }),
    eidolon_level_boosts: (eidolon) => ({
      ...(eidolon >= 3 ? { skill: 2, talent: 2 } : {}),
      ...(eidolon >= 5 ? { ultimate: 2, basic: 1 } : {})
    })
  }
};

