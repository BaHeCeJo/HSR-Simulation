/**
 * @character Acheron
 * @role Main DPS
 * @core_mechanic Slashed Dream Stacks (9) instead of Energy. Stacks are gained when ANY actor inflicts a debuff.
 * @skill_priority Ultimate > Skill > Basic
 * @team_synergies 2 Nihility allies (1 if E2).
 * @eidolon_milestones E1 (Crit Rate), E2 (Nihility Requirement Reduction + Stacks), E4 (Ult DMG Vulnerability), E6 (All DMG as Ult DMG + RES PEN).
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy, Action } from "../types.js";
import { calculateHsrDamage, type SimulationInput } from "../formulas.js";

// Definitive UUIDs from HSR_ID_MAPPING.md
const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const DMG_BOOST_ID = '5169b8ca-b8c5-4bfc-9570-7f194789dfd7';

const ACHERON_ID = "f06222e4-d23d-4ac2-86ff-3a6cc389b812";

/**
 * Helper to get/set Crimson Knot stacks on an enemy
 */
const getCK = (state: SimState, enemyInstanceId: string) => state.stacks[`ck_${enemyInstanceId}`] || 0;
const setCK = (state: SimState, enemyInstanceId: string, value: number) => {
    state.stacks[`ck_${enemyInstanceId}`] = Math.min(Math.max(0, value), 9);
};

const addSD = (state: SimState, member: TeamMember, amount: number, target?: SimEnemy) => {
    const current = state.stacks[member.characterId] || 0;
    const overflow = Math.max(0, (current + amount) - 9);
    state.stacks[member.characterId] = Math.min(current + amount, 9);
    
    // Red Oni Trace: Overflow becomes Quadrivalent Ascendance
    if (overflow > 0) {
        const qa = state.stacks[`qa_${member.characterId}`] || 0;
        state.stacks[`qa_${member.characterId}`] = Math.min(qa + overflow, 3);
    }

    // Apply Crimson Knot to target (or random if none)
    if (target) {
        setCK(state, target.instanceId, getCK(state, target.instanceId) + amount);
    } else {
        const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
        if (aliveEnemies.length > 0) {
            // Inflict on enemy with most CK stacks
            const bestTarget = aliveEnemies.reduce((prev, curr) => getCK(state, curr.instanceId) > getCK(state, prev.instanceId) ? curr : prev);
            setCK(state, bestTarget.instanceId, getCK(state, bestTarget.instanceId) + amount);
        }
    }
};

export const Acheron: CharacterKit = {
  id: ACHERON_ID,
  slot_names: {
    basic: "Trilateral Wiltcross",
    skill: "Octobolt Flash",
    ultimate: "Slashed Dream Cries in Red",
    talent: "Atop Rainleaf Hangs Oneness",
  },
  abilities: {
    basic: {
      ability_id: "ebbbb486-dac6-49b1-ad84-3b6919a14b1f",
      attribute_index: 0,
      default_multiplier: 1.0, // Lv.6
      stat_id: ATK_ID,
      distribution: { hits: [1.0] },
      targetType: 'SingleTarget',
      toughness_damage: 10
    },
    skill: {
      main: { 
        ability_id: "bc988427-c756-44b2-810a-1d696e61b78e",
        attribute_index: 0, 
        default_multiplier: 1.6, // Lv.10
        stat_id: ATK_ID,
        distribution: { hits: [0.1, 0.1, 0.1, 0.7] },
        targetType: 'Blast',
        toughness_damage: 20
      },
      adjacent: { 
        ability_id: "bc988427-c756-44b2-810a-1d696e61b78e",
        attribute_index: 1, 
        default_multiplier: 0.6, // Lv.10
        stat_id: ATK_ID,
        targetType: 'Blast',
        toughness_damage: 10
      }
    },
    ultimate: {
      rainblade_main: { 
        ability_id: "bad43716-de03-4dc7-960b-1ff45133ae06",
        attribute_index: 0, 
        default_multiplier: 0.24, // Lv.10
        stat_id: ATK_ID,
        targetType: 'SingleTarget',
        toughness_damage: 5
      },
      rainblade_adjacent: {
        toughness_damage: 5
      },
      rainblade_aoe: {
        ability_id: "bad43716-de03-4dc7-960b-1ff45133ae06",
        attribute_index: 1,
        default_multiplier: 0.15, // Base Lv.10
        stat_id: ATK_ID,
        targetType: 'AoE'
      },
      stygian_resurge: { 
        ability_id: "bad43716-de03-4dc7-960b-1ff45133ae06",
        attribute_index: 2, 
        default_multiplier: 1.20, // Lv.10
        stat_id: ATK_ID,
        distribution: { hits: [0.1, 0.9] },
        targetType: 'AoE',
        toughness_damage: 10
      },
      thunder_core_extra: {
        default_multiplier: 0.25,
        stat_id: ATK_ID
      }
    },
    talent: {
      res_reduction: {
        default_multiplier: 0.20,
        stat_id: "RES_PEN_ID" 
      }
    }
  },
  hooks: {
    onBattleStart: (state, member) => {
      // Red Oni Trace: Start with 5 Slashed Dream and 5 Crimson Knot on a random enemy
      state.stacks[`qa_${member.characterId}`] = 0;
      const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
      const startTarget = aliveEnemies.length > 0 ? aliveEnemies[Math.floor(Math.random() * aliveEnemies.length)] : undefined;
      addSD(state, member, 5, startTarget);

      // E4: Ult DMG Vulnerability
      if (member.eidolon >= 4) {
        state.enemies.forEach(e => { if (e) e.vulnerability += 8; });
      }
    },
    onTurnStart: (state, member) => {
      // E2: +1 stack at start of turn
      if (member.eidolon >= 2) {
        addSD(state, member, 1);
      }
    },
    onBeforeAction: (state, member, action) => {
      // E1: Crit Rate (18% against debuffed enemies)
      if (member.eidolon >= 1) {
        member.buffs.crit_rate += 18;
      }

      // Trace: The Abyss (Nihility Multiplier)
      const otherNihility = state.nihilityCount - 1;
      const effectiveNihility = member.eidolon >= 2 ? otherNihility + 1 : otherNihility;
      const abyssMult = effectiveNihility >= 2 ? 1.60 : (effectiveNihility >= 1 ? 1.15 : 1.0);
      member.buffs.extra_multiplier += (abyssMult - 1) * 100;

      // Trace: Thunder Core (Persistent DMG boost from previous Ult usage)
      if (state.buffDurations[member.characterId]?.["thunder_core"]) {
        member.buffs.dmg_boost += state.buffDurations[member.characterId]["thunder_core"].value || 0;
      }

      // E6 / Acheron logic: Skill/Basic count as Ultimate DMG
      if (action.type === 'ultimate' || member.eidolon >= 6) {
        action.is_ult_dmg = true;
        action.inflictsDebuff = true;
        // Acheron inherently has RES PEN during Ult (handled in onUlt)
        if (member.eidolon >= 6) {
          member.buffs.res_pen += 20;
        }
      } else if (action.type === 'skill') {
        action.inflictsDebuff = true;
        // Acheron's Skill grants 1 point directly (Red Oni / Talent logic)
        const skillKey = `acheron_skill_gain_${member.characterId}`;
        if (state.stacks[skillKey] !== state.currentActionId) {
            addSD(state, member, 1, state.enemy); // Inflict CK on main target
            state.stacks[skillKey] = state.currentActionId as any;
            state.addLog({ type: 'event', message: `Acheron gains 1 Slashed Dream from Skill (Total: ${state.stacks[member.characterId]})` });
        }
      }
    },
    onUlt: (state, member) => {
      let totalUltDmg = 0;
      const ultAbilities = Acheron.abilities.ultimate as any;
      
      // --- START ULTIMATE FIELD EFFECTS ---
      // Talent: Reduce All-Type RES by 20%
      state.enemies.forEach(e => { if (e) e.resistance -= 0.20; });
      
      // Temporary state for the buff that builds DURING the Ult (Thunder Core)
      const thunderCoreKey = "thunder_core_active";
      let tcStacks = 0;
      const baseDmgBoost = member.buffs.dmg_boost;

      // 1. Sequentially fire 3 Rainblades
      for (let i = 1; i <= 3; i++) {
        const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
        if (aliveEnemies.length === 0) break;

        // Target enemy with most Crimson Knots, or main target
        const mainTarget = aliveEnemies.reduce((prev, curr) => getCK(state, curr.instanceId) > getCK(state, prev.instanceId) ? curr : prev) || state.enemy;
        const mainTargetIdx = state.enemies.indexOf(mainTarget);
        
        // Remove up to 3 Crimson Knots
        const ckCount = getCK(state, mainTarget.instanceId);
        const removed = Math.min(ckCount, 3);
        setCK(state, mainTarget.instanceId, ckCount - removed);

        // Thunder Core: DMG increases by 30% when hitting targets with Crimson Knot
        if (ckCount > 0) {
            tcStacks = Math.min(tcStacks + 1, 3);
        }
        member.buffs.dmg_boost = baseDmgBoost + (tcStacks * 30);

        // A. Rainblade ST Damage
        const rbResult = calculateHsrDamage({
            character: member,
            lightcone: member.lightcone,
            enemy: mainTarget,
            ability_multiplier: ultAbilities.rainblade_main.default_multiplier,
            scaling_stat_id: ATK_ID
        });
        state.addLog({ type: 'event', message: `Hit: Rainblade ${i} on ${mainTarget.name} -> ${rbResult.expected_dmg.toLocaleString()} DMG (Removed ${removed} CK)` });
        totalUltDmg += rbResult.expected_dmg;
        mainTarget.hp = Math.max(0, Math.floor(mainTarget.hp - rbResult.expected_dmg));

        // Toughness: Rainblade reduces 5 to main and 5 to adjacent (regardless of weakness)
        if ((state as any).applyToughnessDamage) {
            (state as any).applyToughnessDamage(mainTarget, 5, true);
        }

        // B. Rainblade AOE Damage (Knot removal trigger)
        // Multiplier: 15% base + 15% per stack removed (up to 60%)
        const aoeMult = 0.15 + (removed * 0.15);
        state.enemies.forEach((e, idx) => {
            if (e && e.hp > 0) {
                const aoeResult = calculateHsrDamage({
                    character: member,
                    lightcone: member.lightcone,
                    enemy: e,
                    ability_multiplier: aoeMult,
                    scaling_stat_id: ATK_ID
                });
                totalUltDmg += aoeResult.expected_dmg;
                e.hp = Math.max(0, Math.floor(e.hp - aoeResult.expected_dmg));

                // Adjacent Toughness reduction (5)
                if (e !== mainTarget && Math.abs(idx - mainTargetIdx) === 1) {
                    if ((state as any).applyToughnessDamage) {
                        (state as any).applyToughnessDamage(e, 5, true);
                    }
                }
            }
        });
      }

      // 2. Fire Stygian Resurge (AoE)
      const aliveAfterRain = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
      if (aliveAfterRain.length > 0) {
          const srMult = ultAbilities.stygian_resurge.default_multiplier;
          aliveAfterRain.forEach(enemy => {
              const srResult = calculateHsrDamage({
                  character: member,
                  lightcone: member.lightcone,
                  enemy: enemy,
                  ability_multiplier: srMult,
                  scaling_stat_id: ATK_ID
              });
              state.addLog({ type: 'event', message: `Hit: Stygian Resurge on ${enemy.name} -> ${srResult.expected_dmg.toLocaleString()} DMG` });
              totalUltDmg += srResult.expected_dmg;
              enemy.hp = Math.max(0, Math.floor(enemy.hp - srResult.expected_dmg));
              
              // Toughness: 10 to all (regardless of weakness)
              if ((state as any).applyToughnessDamage) {
                  (state as any).applyToughnessDamage(enemy, 10, true);
              }

              // Remove all remaining Crimson Knots
              setCK(state, enemy.instanceId, 0);
          });
      }

      // 3. Thunder Core: 6 Extra Hits (AoE random)
      if (aliveAfterRain.length > 0) {
          for (let k = 0; k < 6; k++) {
              const targets = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
              if (targets.length === 0) break;
              const randomTarget = targets[Math.floor(Math.random() * targets.length)];
              const extraResult = calculateHsrDamage({
                  character: member,
                  lightcone: member.lightcone,
                  enemy: randomTarget,
                  ability_multiplier: 0.25,
                  scaling_stat_id: ATK_ID
              });
              totalUltDmg += extraResult.expected_dmg;
              randomTarget.hp = Math.max(0, Math.floor(randomTarget.hp - extraResult.expected_dmg));
          }
          state.addLog({ type: 'event', message: `Hit: Thunder Core 6-hit finisher triggered.` });
      }

      // --- END ULTIMATE FIELD EFFECTS ---
      state.enemies.forEach(e => { if (e) e.resistance += 0.20; });
      member.buffs.dmg_boost = baseDmgBoost; // Restore original

      // Update State
      state.totalDamage += totalUltDmg;
      
      // Quadrivalent Ascendance Refund
      const qaStacks = state.stacks[`qa_${member.characterId}`] || 0;
      state.stacks[member.characterId] = 0; // Reset SD
      state.stacks[`qa_${member.characterId}`] = 0; // Reset QA
      
      if (qaStacks > 0) {
          state.addLog({ type: 'event', message: `Acheron converts ${qaStacks} Quadrivalent Ascendance stacks into Slashed Dream.` });
          addSD(state, member, qaStacks);
      }

      // Persistence: Thunder Core buff lasts 3 turns
      if (tcStacks > 0) {
          state.buffDurations[member.characterId]["thunder_core"] = { duration: 3, value: tcStacks * 30, stat: "DMG Boost" };
      }
    },
    onGlobalDebuff: (state, source, target) => {
      // Ensure only one stack is gained per action
      const actionKey = `acheron_last_action_${ACHERON_ID}`;
      if (state.stacks[actionKey] === state.currentActionId) return;
      state.stacks[actionKey] = state.currentActionId as any;

      addSD(state, memberFromTeam(state, ACHERON_ID)!, 1, target);
      state.addLog({ type: 'event', message: `Acheron gains 1 Slashed Dream from Talent via ${source.name}'s action (Total: ${state.stacks[ACHERON_ID]})` });
    },
    onEnemyDefeated: (state, member, enemy) => {
        const ck = getCK(state, enemy.instanceId);
        if (ck > 0) {
            const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
            if (aliveEnemies.length > 0) {
                // Priority: Enemy with most CK stacks > Boss > Elite > Normal
                const target = aliveEnemies.reduce((prev, curr) => {
                    const ckP = getCK(state, prev.instanceId);
                    const ckC = getCK(state, curr.instanceId);
                    if (ckC > ckP) return curr;
                    if (ckC < ckP) return prev;
                    // Tie-break: Boss/Elite (simulated by checking if HP/SPD is significantly higher, or just fallback)
                    return curr; 
                });
                setCK(state, target.instanceId, getCK(state, target.instanceId) + ck);
                state.addLog({ type: 'event', message: `Transferred ${ck} Crimson Knots to ${target.name}.` });
            }
        }
    }
  },
  special_modifiers: {
    energy_type: "STACKS", 
    stat_boosts: (state: any) => ({
      atk_percent: 28,
      dmg_boost: 8,
      crit_dmg: 24
    }),
    eidolon_level_boosts: (eidolon: number) => ({
      ...(eidolon >= 3 ? { ultimate: 2, basic: 1 } : {}),
      ...(eidolon >= 5 ? { skill: 2, talent: 2 } : {})
    })
  }
};

function memberFromTeam(state: SimState, id: string) {
    return state.team.find(m => m.characterId === id);
}
