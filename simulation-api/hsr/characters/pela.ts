/**
 * @character Pela
 * @role Support / Debuffer
 * @core_mechanic AOE DEF Reduction via Ultimate. Energy refund via Talent when attacking debuffed enemies.
 * @skill_priority Ultimate > Basic (Skill only for buff removal)
 * @team_synergies Acheron (Nihility stack), any Main DPS (DEF Shred).
 * @eidolon_milestones E4 (Ice RES Reduction), E6 (Additional DMG to debuffed enemies).
 */

import type { CharacterKit } from "../types.js";

const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';

export const Pela: CharacterKit = {
  id: "a8a4d435-bcd4-4105-83ab-72650f296844",
  slot_names: {
    basic: "Frost Shot",
    skill: "Frostbite",
    ultimate: "Zone Suppression",
    talent: "Data Collecting",
  },
  abilities: {
    basic: { default_multiplier: 1.0, stat_id: ATK_ID, toughness_damage: 10 },
    skill: { default_multiplier: 2.1, stat_id: ATK_ID, toughness_damage: 20 },
    ultimate: { default_multiplier: 1.0, stat_id: ATK_ID, toughness_damage: 20 },
    talent: { default_multiplier: 0.1, stat_id: "ENERGY" },
  },
  hooks: {
    onTurnStart: (state, member) => {
        // Pela naturally gets energy or other buffs
    },
    onBeforeAction: (state, member, action) => {
      if (action.type === 'ultimate') {
        action.inflictsDebuff = true;
        state.enemy.activeDebuffs["pela_ult_def"] = { duration: 2, value: 40, stat: "DEF Reduction" }; // 40% DEF Reduction
        state.enemy.debuffCount = Object.keys(state.enemy.activeDebuffs).length;
      }
      if (action.type === 'skill') {
          action.inflictsDebuff = true; // Remove buff is considered debuff in some contexts or just for stacks
      }
    }
  },
  special_modifiers: {
    energy_type: "ENERGY",
    energy_cost: 110,
    stat_boosts: (state) => ({
      atk_percent: 18,
      dmg_boost: 10
    }),
    eidolon_level_boosts: (eidolon) => ({})
  }
};
