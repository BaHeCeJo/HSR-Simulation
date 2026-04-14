/**
 * @character Argenti
 * @role Main DPS (Erudition / Physical)
 * @core_mechanic Apotheosis stacks grant CRIT Rate (+CRIT DMG at E1).
 *   Two ultimates share the same energy resource:
 *     - 90 energy  ↁE"For In This Garden Supreme Beauty Bestows" (160% AoE, toughness 20)
 *     - 180 energy ↁE'Merit Bestowed in "My" Garden' (280% AoE + 6ÁE5% random hits, toughness 20)
 *   By default the sim waits for 180. Set state.stacks['argenti_prefer_90_ult'] = 1 to fire at 90.
 * @skill_priority Ultimate > Skill > Basic
 * @eidolon_milestones E1 (CRIT DMG/stack), E2 (ATK on AoE ult), E4 (extra stacks + cap), E6 (DEF ignore on ult).
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const ARGENTI_ID = "e1a2b3c4-d5e6-4f7a-8b9c-0d1e2f3a4b5c";
const APO_KEY = 'argenti_apotheosis';
const ENERGY_KEY = 'argenti_energy'; // actual energy counter (0 E80+)

function getApo(state: SimState): number {
  return state.stacks[APO_KEY] || 0;
}

function addApo(state: SimState, member: TeamMember, count: number): void {
  const max = member.eidolon >= 4 ? 12 : 10;
  state.stacks[APO_KEY] = Math.min((state.stacks[APO_KEY] || 0) + count, max);
}

/** Adds energy and sets the ult trigger when the appropriate threshold is reached. */
function addEnergy(state: SimState, amount: number): void {
  state.stacks[ENERGY_KEY] = (state.stacks[ENERGY_KEY] || 0) + amount;

  const prefer90 = state.stacks['argenti_prefer_90_ult'] || 0;
  const threshold = prefer90 ? 90 : 180;

  if (state.stacks[ENERGY_KEY] >= threshold) {
    // Signal the simulator that the ult is ready (checked as stacks[id] >= energy_cost)
    state.stacks[ARGENTI_ID] = 90;
  }
}

export const Argenti: CharacterKit = {
  id: ARGENTI_ID,
  name: "Argenti",
  path: "Erudition",
  element: "Physical",
  slot_names: {
    basic: "Fleeting Fragrance",
    skill: "Justice, Hereby Blooms",
    ultimate: 'Argenti Ultimate',   // resolved to 90 or 180 version at runtime
    talent: "Sublime Object",
  },
  abilities: {
    basic: {
      default_multiplier: 1.0, // Lv.6
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 10
    },
    skill: {
      default_multiplier: 1.2, // Lv.10
      stat_id: ATK_ID,
      targetType: 'AoE',
      toughness_damage: 10
    },
    ultimate: {
      // Default multiplier for the 90-energy version; 180-version uses 2.8 computed inside onUlt
      default_multiplier: 1.6, // Lv.10 (90-energy version)
      stat_id: ATK_ID,
      targetType: 'AoE',
      toughness_damage: 20
    },
    talent: {
      default_multiplier: 0,
      stat_id: ATK_ID
    }
  },
  hooks: {
    onBattleStart: (state, member) => {
      state.stacks[APO_KEY] = 0;
      state.stacks[ENERGY_KEY] = 0;
      state.stacks[ARGENTI_ID] = 0;

      // E4: Gains 2 Apotheosis stacks at start of battle; max cap becomes 12
      if (member.eidolon >= 4) {
        addApo(state, member, 2);
      }

      // A4: +2 Energy per enemy entering battle (initial wave)
      const initialEnemies = state.enemies.filter(e => e !== null).length;
      // Use raw stacks directly  Ethreshold check not needed at battle start
      state.stacks[ENERGY_KEY] = (state.stacks[ENERGY_KEY] || 0) + initialEnemies * 2;
      state.addLog({ type: 'event', message: `Argenti A4: +${initialEnemies * 2} Energy (${initialEnemies} enemies, Total: ${state.stacks[ENERGY_KEY]})` });
    },

    onTurnStart: (state, member) => {
      // A2: +1 Apotheosis at the start of each turn
      addApo(state, member, 1);
      state.addLog({ type: 'event', message: `Argenti A2: +1 Apotheosis (Total: ${getApo(state)})` });
    },

    onBeforeAction: (state, member, action, target) => {
      const apo = getApo(state);

      // Talent: Each Apotheosis stack ↁE+2.5% CRIT Rate
      member.buffs.crit_rate += apo * 2.5;

      // E1: Each Apotheosis stack ↁE+4% CRIT DMG
      if (member.eidolon >= 1) {
        member.buffs.crit_dmg += apo * 4;
      }

      // E2: 3+ enemies alive when using Ultimate ↁE+40% ATK for this action
      if (member.eidolon >= 2 && action.type === 'ultimate') {
        const alive = state.enemies.filter(e => e && e.hp > 0).length;
        if (alive >= 3) {
          member.buffs.atk_percent += 40;
        }
      }

      // E6: Ultimate ignores 30% DEF
      if (member.eidolon >= 6 && action.type === 'ultimate') {
        member.buffs.def_ignore += 30;
      }

      // Cache alive count before the action for talent energy/stack calc in onAfterAction
      state.stacks['argenti_pre_action_alive'] = state.enemies.filter(e => e && e.hp > 0).length;
    },

    onAfterAction: (state, member, action, target) => {
      // Ult talent procs are handled inside onUlt
      if (action.type === 'ultimate') return;

      // Enemies hit: 1 for Basic (SingleTarget), all pre-action alive for Skill (AoE)
      const preAlive = state.stacks['argenti_pre_action_alive'] || 1;
      const enemiesHit = action.targetType === 'SingleTarget' ? 1 : Math.max(1, preAlive);

      // Talent: +3 Energy and +1 Apotheosis per enemy hit
      const talentEnergy = enemiesHit * 3;
      addApo(state, member, enemiesHit);

      // Standard energy gain (Basic: 20, Skill: 30) + talent
      const stdEnergy = action.type === 'basic' ? 20 : 30;
      addEnergy(state, stdEnergy + talentEnergy);

      state.addLog({
        type: 'event',
        message: `Argenti: +${stdEnergy + talentEnergy} Energy (${stdEnergy} base + ${talentEnergy} talent/${enemiesHit} hit(s), Total: ${state.stacks[ENERGY_KEY]}) | Apotheosis: ${getApo(state)}`
      });
    },

    onUlt: (state, member) => {
      const actualEnergy = state.stacks[ENERGY_KEY] || 0;
      const use180 = actualEnergy >= 180;

      // Reset energy; simulator won't auto-reset since energy_type is NONE + onUlt is defined
      state.stacks[ENERGY_KEY] = 0;
      state.stacks[ARGENTI_ID] = 0;

      const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
      if (aliveEnemies.length === 0) return;

      const baseDmgBoost = member.buffs.dmg_boost;
      let totalUltDmg = 0;

      if (use180) {
        // ── 180-energy: Merit Bestowed in "My" Garden ─────────────────────────
        state.addLog({ type: 'event', message: `Using 180-energy Ultimate: Merit Bestowed in "My" Garden` });

        const mainHitCount = aliveEnemies.length;

        // 1. Main AoE: 280% ATK to all enemies
        aliveEnemies.forEach(enemy => {
          // A6: +15% DMG if enemy HP ≤ 50%
          member.buffs.dmg_boost = baseDmgBoost + (enemy.hp / enemy.max_hp <= 0.5 ? 15 : 0);

          const result = calculateHsrDamage({
            character: member,
            lightcone: member.lightcone,
            enemy,
            ability_multiplier: 2.8,
            scaling_stat_id: ATK_ID
          });
          totalUltDmg += result.expected_dmg;
          enemy.hp = Math.max(0, Math.floor(enemy.hp - result.expected_dmg));
          state.addLog({ type: 'event', message: `Hit: Main AoE on ${enemy.name} -> ${result.expected_dmg.toLocaleString()} DMG` });

          if ((state as any).applyToughnessDamage) {
            (state as any).applyToughnessDamage(enemy, 20, false);
          }
        });

        // 2. 6 extra random hits: 95% ATK each
        let extraHitsLanded = 0;
        for (let i = 0; i < 6; i++) {
          const targets = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
          if (targets.length === 0) break;

          const randomTarget = targets[Math.floor(Math.random() * targets.length)];
          member.buffs.dmg_boost = baseDmgBoost + (randomTarget.hp / randomTarget.max_hp <= 0.5 ? 15 : 0);

          const result = calculateHsrDamage({
            character: member,
            lightcone: member.lightcone,
            enemy: randomTarget,
            ability_multiplier: 0.95,
            scaling_stat_id: ATK_ID
          });
          totalUltDmg += result.expected_dmg;
          randomTarget.hp = Math.max(0, Math.floor(randomTarget.hp - result.expected_dmg));
          extraHitsLanded++;
        }

        state.addLog({ type: 'event', message: `Merit Bestowed: ${extraHitsLanded}/6 extra hits landed.` });

        // Talent procs: main AoE + extra hits
        const totalHits = mainHitCount + extraHitsLanded;
        addApo(state, member, totalHits);
        const postEnergy = 5 + totalHits * 3;
        addEnergy(state, postEnergy);
        state.addLog({ type: 'event', message: `Post-ult: +${totalHits} Apotheosis (Total: ${getApo(state)}), +${postEnergy} Energy (Total: ${state.stacks[ENERGY_KEY]})` });

      } else {
        // ── 90-energy: For In This Garden Supreme Beauty Bestows ──────────────
        state.addLog({ type: 'event', message: `Using 90-energy Ultimate: For In This Garden Supreme Beauty Bestows` });

        // AoE: 160% ATK to all enemies
        aliveEnemies.forEach(enemy => {
          // A6: +15% DMG if enemy HP ≤ 50%
          member.buffs.dmg_boost = baseDmgBoost + (enemy.hp / enemy.max_hp <= 0.5 ? 15 : 0);

          const result = calculateHsrDamage({
            character: member,
            lightcone: member.lightcone,
            enemy,
            ability_multiplier: 1.6,
            scaling_stat_id: ATK_ID
          });
          totalUltDmg += result.expected_dmg;
          enemy.hp = Math.max(0, Math.floor(enemy.hp - result.expected_dmg));
          state.addLog({ type: 'event', message: `Hit: AoE on ${enemy.name} -> ${result.expected_dmg.toLocaleString()} DMG` });

          if ((state as any).applyToughnessDamage) {
            (state as any).applyToughnessDamage(enemy, 20, false);
          }
        });

        // Talent procs: one hit per enemy
        const hitsLanded = aliveEnemies.length;
        addApo(state, member, hitsLanded);
        const postEnergy = 5 + hitsLanded * 3;
        addEnergy(state, postEnergy);
        state.addLog({ type: 'event', message: `Post-ult: +${hitsLanded} Apotheosis (Total: ${getApo(state)}), +${postEnergy} Energy (Total: ${state.stacks[ENERGY_KEY]})` });
      }

      member.buffs.dmg_boost = baseDmgBoost; // Restore
      state.totalDamage += totalUltDmg;
    }
  },

  special_modifiers: {
    energy_type: "NONE",  // Energy tracked in ENERGY_KEY; ult trigger set manually in addEnergy()
    energy_cost: 90,      // Simulator checks stacks[id] >= 90; threshold (90 vs 180) is in addEnergy()
    stat_boosts: (state: any) => ({
      atk_percent: 28,   // Minor trace: ATK +28%
      dmg_boost: 14.4,   // Minor trace: Physical DMG +14.4%
    }),
    eidolon_level_boosts: (eidolon: number) => ({
      ...(eidolon >= 3 ? { skill: 2, talent: 2 } : {}),
      ...(eidolon >= 5 ? { ultimate: 2, basic: 1 } : {})
    })
  }
};
