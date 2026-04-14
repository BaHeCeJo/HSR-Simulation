/**
 * @character Jiaoqiu
 * @role Support / Debuffer / DoT
 * @core_mechanic Ashen Roast stacks (Vulnerability + Burn DoT). Ultimate Zone for Ult DMG boost and stack generation.
 * @skill_priority Ultimate > Skill > Basic
 * @team_synergies Acheron (Fast stacks), DoT teams (E2).
 * @eidolon_milestones E1 (DMG Boost + Faster Stacks), E2 (Massive DoT), E6 (RES PEN + 9 Stacks).
 */

import type { CharacterKit, Action, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const DMG_BOOST_ID = '5169b8ca-b8c5-4bfc-9570-7f194789dfd7';

export const Jiaoqiu: CharacterKit = {
  id: "f06222e4-d23d-4ac2-86ff-3a6cc389b813",
  slot_names: {
    basic: "Heart Afire",
    skill: "Scorch Onslaught",
    ultimate: "Pyrograph Arcanum",
    talent: "Quartet Finesse, Octave Finery",
  },
  abilities: {
    basic: {
      ability_id: "ABILITY_ID_PLACEHOLDER",
      attribute_index: 0,
      default_multiplier: 1.0, // {100%}
      stat_id: ATK_ID,
      toughness_damage: 10
    },
    skill: {
      main: {
        ability_id: "ABILITY_ID_PLACEHOLDER",
        attribute_index: 0,
        default_multiplier: 1.5, // {150%}
        stat_id: ATK_ID,
        toughness_damage: 20
      },
      adjacent: {
        ability_id: "ABILITY_ID_PLACEHOLDER",
        attribute_index: 1,
        default_multiplier: 0.9, // {90%}
        stat_id: ATK_ID,
        toughness_damage: 10
      }
    },
    ultimate: {
      main: {
        ability_id: "ABILITY_ID_PLACEHOLDER",
        attribute_index: 0,
        default_multiplier: 1.0, // {100%}
        stat_id: ATK_ID,
        toughness_damage: 20
      },
      ult_dmg_vuln: {
        ability_id: "ABILITY_ID_PLACEHOLDER",
        attribute_index: 1,
        default_multiplier: 0.15, // {15%}
        stat_id: "VULN_ID"
      }
    },
    talent: {
      dot: {
        ability_id: "ABILITY_ID_PLACEHOLDER",
        attribute_index: 0,
        default_multiplier: 1.8, // {180%}
        stat_id: ATK_ID
      },
      vuln_base: {
        ability_id: "ABILITY_ID_PLACEHOLDER",
        attribute_index: 1,
        default_multiplier: 0.15, // {15%}
        stat_id: "VULN_ID"
      },
      vuln_stack: {
        ability_id: "ABILITY_ID_PLACEHOLDER",
        attribute_index: 2,
        default_multiplier: 0.05, // {5%}
        stat_id: "VULN_ID"
      }
    }
  },
  hooks: {
    onBattleStart: (state, member) => {
      // A2: Start battle with +15 energy
      state.stacks[member.characterId] = (state.stacks[member.characterId] || 0) + 15;
    },
    onTurnStart: (state, member) => {
      // Zone duration decrease
      if (state.buffDurations[member.characterId]?.["jiaoqiu_zone"] > 0) {
        state.buffDurations[member.characterId]["jiaoqiu_zone"]--;
        if (state.buffDurations[member.characterId]["jiaoqiu_zone"] === 0) {
            state.stacks["jiaoqiu_zone_triggers"] = 0;
        }
      }
    },
    onBeforeAction: (state, member, action, target) => {
      // A4: EHR to ATK conversion
      const ehr = 140; // Placeholder
      if (ehr > 80) {
          const extraAtk = Math.min(Math.floor((ehr - 80) / 15) * 60, 240);
          member.buffs.atk_percent += extraAtk;
      }

      // 1. Calculate Vulnerability from Ashen Roast
      if (target) {
        const stacks = state.stacks[`ashen_roast_${target.instanceId}`] || 0;
        if (stacks > 0) {
            let vuln = 15 + (stacks - 1) * 5; // Base 15% + 5% per stack
            if (member.eidolon >= 1) member.buffs.dmg_boost += 40;
            target.vulnerability += vuln;
            if (member.eidolon >= 6) member.buffs.res_pen += stacks * 3;
        }

        // 2. Zone: 15% Ult DMG Vulnerability
        if (state.buffDurations[member.characterId]?.["jiaoqiu_zone"] > 0 && action.is_ult_dmg) {
            target.vulnerability += 15;
        }
      }

      action.inflictsDebuff = true;
    },
    onAfterAction: (state, member, action, target) => {
      if (!target) return;
      if (action.type === 'basic' || action.type === 'skill' || action.type === 'ultimate') {
          const maxStacks = member.eidolon >= 6 ? 9 : 5;
          const currentStacks = state.stacks[`ashen_roast_${target.instanceId}`] || 0;
          let addedStacks = 1;
          if (member.eidolon >= 1) addedStacks += 1;
          
          state.stacks[`ashen_roast_${target.instanceId}`] = Math.min(currentStacks + addedStacks, maxStacks);
          target.activeDebuffs["ashen_roast"] = { 
              duration: 2, 
              value: state.stacks[`ashen_roast_${target.instanceId}`],
              stat: "Ashen Roast Stacks" 
          };
      }
    },
    onUlt: (state, member) => {
      // Equalize stacks on ALL enemies
      let maxOnField = 0;
      state.enemies.forEach(e => {
          if (e) maxOnField = Math.max(maxOnField, state.stacks[`ashen_roast_${e.instanceId}`] || 0);
      });

      state.enemies.forEach(e => {
          if (e) {
            state.stacks[`ashen_roast_${e.instanceId}`] = Math.max(maxOnField, 1);
            e.activeDebuffs["ashen_roast"] = { 
                duration: 2, 
                value: state.stacks[`ashen_roast_${e.instanceId}`],
                stat: "Ashen Roast Stacks"
            };
          }
      });
      
      state.buffDurations[member.characterId] = state.buffDurations[member.characterId] || {};
      state.buffDurations[member.characterId]["jiaoqiu_zone"] = 3;
      state.stacks["jiaoqiu_zone_triggers"] = 6;
      state.stacks[member.characterId] = 0;
    },
    onEnemyTurnStart: (state, member, enemy) => {
        const stacks = state.stacks[`ashen_roast_${enemy.instanceId}`] || 0;
        if (stacks > 0) {
            let multiplier = 1.8;
            if (member.eidolon >= 2) multiplier += 3.0;

            const dotResult = calculateHsrDamage({
                character: member,
                lightcone: member.lightcone,
                enemy: enemy,
                ability_multiplier: multiplier,
                scaling_stat_id: ATK_ID
            });
            state.totalDamage += dotResult.expected_dmg;
        }
    },
    onEnemyAction: (state, member, enemy) => {
        if (state.buffDurations[member.characterId]?.["jiaoqiu_zone"] > 0 && (state.stacks["jiaoqiu_zone_triggers"] || 0) > 0) {
            const maxStacks = member.eidolon >= 6 ? 9 : 5;
            const currentStacks = state.stacks[`ashen_roast_${enemy.instanceId}`] || 0;
            
            if (currentStacks < maxStacks) {
                state.stacks[`ashen_roast_${enemy.instanceId}`]++;
                state.stacks["jiaoqiu_zone_triggers"]--;
                enemy.activeDebuffs["ashen_roast"] = { 
                    duration: 2, 
                    value: state.stacks[`ashen_roast_${enemy.instanceId}`],
                    stat: "Ashen Roast Stacks"
                };

                // Trigger GlobalDebuff for Acheron (manual trigger as this is outside of a character's direct action)
                state.team.forEach(m => {
                    const k = m.characterId === "f06222e4-d23d-4ac2-86ff-3a6cc389b812" ? 
                        // We need Acheron's kit but we can't import registry.
                        // However, onGlobalDebuff is triggered by simulator usually.
                        // Here it is a Jiaoqiu's passive trigger.
                        // We'll leave it for now or find another way.
                        // Actually, Acheron's talent is triggered when ANY actor inflicts a debuff.
                        // Jiaoqiu's zone inflicts a debuff.
                        null : null;
                });
            }
        }
    }
  },
  special_modifiers: {
    energy_type: "ENERGY",
    energy_cost: 100,
    stat_boosts: (state) => ({
      atk_percent: 28,
      fire_dmg: 14.4,
      spd: 5
    }),
    eidolon_level_boosts: (eidolon) => ({
      ...(eidolon >= 3 ? { skill: 2, basic: 1 } : {}),
      ...(eidolon >= 5 ? { ultimate: 2, talent: 2 } : {})
    })
  }
};
