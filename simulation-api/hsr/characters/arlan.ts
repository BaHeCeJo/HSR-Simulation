/**
 * @character Arlan
 * @role Main DPS (Destruction / Lightning)
 * @core_mechanic Skill costs 15% Max HP; Talent converts missing HP into DMG bonus (up to 72%).
 *   Lower HP ↁEhigher damage, creating a high-risk/high-reward loop.
 * @skill_priority Skill > Ultimate > Basic
 * @eidolon_milestones E1 (Skill +10% DMG at ≤50% HP), E2 (debuff removal), E4 (survival), E6 (Ult adj=main at ≤50% HP).
 *
 * Defensive mechanics implemented:
 *   A4  E+50% chance to resist DoT debuffs (tracked as activeBuffs flag; checked by enemy kits)
 *   A6  EOn battle entry with HP ≤ 50%, nullify all non-DoT DMG until first hit (via applyDamageToAlly wrap)
 *   E2  ESkill/Ultimate removes 1 debuff from Arlan
 *   E4  ESurvive one killing blow per battle (HPↁE5% instead of 0); expires after 2 turns or on use
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const ARLAN_ID = "d2c3b4a5-e6f7-4a8b-9c0d-1e2f3a4b5c6d";

function getArlan(state: SimState): TeamMember | undefined {
  return state.team.find(m => m.characterId === ARLAN_ID);
}

/** Remove the oldest debuff from Arlan (E2). */
function removeOneDebuff(state: SimState): void {
  const original = getArlan(state);
  if (!original) return;
  const keys = Object.keys(original.activeDebuffs);
  if (keys.length === 0) return;
  const removed = keys[0];
  delete original.activeDebuffs[removed];
  state.addLog({ type: 'event', message: `Arlan E2: Removed debuff [${removed}]` });
}

export const Arlan: CharacterKit = {
  id: ARLAN_ID,
  name: "Arlan",
  path: "Destruction",
  element: "Lightning",
  slot_names: {
    basic: "Lightning Rush",
    skill: "Shackle Breaker",
    ultimate: "Frenzied Punishment",
    talent: "Pain and Anger",
  },
  abilities: {
    basic: {
      default_multiplier: 1.0, // Lv.6
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 10
    },
    skill: {
      default_multiplier: 2.4, // Lv.10
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 20
    },
    ultimate: {
      main: {
        default_multiplier: 3.2, // Lv.10
        stat_id: ATK_ID,
        targetType: 'Blast',
        toughness_damage: 20
      },
      adjacent: {
        default_multiplier: 1.6, // Lv.10 (becomes 3.2 at E6 when HP ≤ 50%)
        stat_id: ATK_ID,
        targetType: 'Blast',
        toughness_damage: 20
      }
    },
    talent: {
      default_multiplier: 0,
      stat_id: ATK_ID
    }
  },
  hooks: {
    onBattleStart: (state, member) => {
      const original = getArlan(state);

      // ── A4: +50% DoT resist  Estored as activeBuffs flag for enemy kits to check ──
      if (original) {
        original.activeBuffs['arlan_dot_resist'] = { duration: 9999, value: 50, stat: 'Effect RES (DoT)' };
        state.addLog({ type: 'event', message: `Arlan A4: DoT Resist +50% active.` });
      }

      // ── A6: Nullify all non-DoT DMG until first hit, if entering with HP ≤ 50% ──
      if (original && original.hp / original.max_hp <= 0.5) {
        state.stacks['arlan_a6_active'] = 1;
        original.activeBuffs['arlan_a6_shield'] = { duration: 9999, stat: 'DMG Nullification (A6)' };
        state.addLog({ type: 'event', message: `Arlan A6: Entering battle at ≤50% HP  EDMG nullification active until first hit.` });
      }

      // ── E4: Survive one killing blow; effect expires after 2 turns ──
      if (member.eidolon >= 4) {
        state.stacks['arlan_e4_active'] = 1;
        state.stacks['arlan_e4_turns_left'] = 2;
        state.addLog({ type: 'event', message: `Arlan E4: Survival effect active (2 turns).` });
      }

      // ── Wrap applyDamageToAlly to handle A6 nullification and E4 survival ──
      const originalApply = state.applyDamageToAlly!;
      state.applyDamageToAlly = (target: TeamMember, damage: number, toughnessDamage?: number) => {
        if (target.characterId === ARLAN_ID) {
          // A6: Nullify first non-DoT hit (damage channel, not DoT periodic tick)
          if (state.stacks['arlan_a6_active']) {
            state.stacks['arlan_a6_active'] = 0;
            delete target.activeBuffs['arlan_a6_shield'];
            state.addLog({ type: 'event', message: `Arlan A6: Incoming ${damage.toLocaleString()} DMG nullified. Effect consumed.` });
            // Still apply toughness (A6 only blocks damage, not toughness reduction)
            if (toughnessDamage && toughnessDamage > 0) originalApply(target, 0, toughnessDamage);
            return;
          }

          // E4: Convert a killing blow into HP restoration to 25% Max HP
          if (member.eidolon >= 4 && state.stacks['arlan_e4_active'] && target.hp - damage <= 0) {
            state.stacks['arlan_e4_active'] = 0;
            target.hp = Math.floor(target.max_hp * 0.25);
            state.addLog({ type: 'event', message: `Arlan E4: Survived killing blow! HP restored to 25% (${Math.floor(target.hp)}/${Math.floor(target.max_hp)})` });
            // Apply toughness reduction even though HP is saved
            if (toughnessDamage && toughnessDamage > 0) originalApply(target, 0, toughnessDamage);
            return;
          }
        }
        originalApply(target, damage, toughnessDamage);
      };
    },

    onTurnStart: (state, member) => {
      // E4: Decrement turn counter and expire if elapsed
      if (member.eidolon >= 4 && state.stacks['arlan_e4_active']) {
        state.stacks['arlan_e4_turns_left']--;
        if (state.stacks['arlan_e4_turns_left'] <= 0) {
          state.stacks['arlan_e4_active'] = 0;
          state.addLog({ type: 'event', message: `Arlan E4: Survival effect expired after 2 turns.` });
        }
      }
    },

    onBeforeAction: (state, member, action, target) => {
      // ── Skill: consume 15% Max HP BEFORE talent calc (more missing HP = more DMG) ──
      if (action.type === 'skill') {
        const original = getArlan(state);
        if (original) {
          const cost = original.max_hp * 0.15;
          if (original.hp <= cost) {
            original.hp = 1; // Never drops below 1
          } else {
            original.hp -= cost;
          }
          member.hp = original.hp; // Sync into actionMember for the talent calc below
          state.addLog({
            type: 'event',
            message: `Arlan Skill HP cost: -${Math.floor(cost)} HP (HP: ${Math.floor(original.hp)}/${Math.floor(original.max_hp)})`
          });
        }
      }

      // ── Talent (Pain and Anger): +0.72% DMG per 1% missing HP, up to 72% ──
      const hpPct = member.max_hp > 0 ? member.hp / member.max_hp : 1;
      const talentBoost = (1 - hpPct) * 72;
      member.buffs.dmg_boost += talentBoost;

      // ── E1: Skill +10% DMG when HP ≤ 50% ──
      if (member.eidolon >= 1 && action.type === 'skill' && hpPct <= 0.5) {
        member.buffs.dmg_boost += 10;
      }

      // ── E6: Ultimate +20% DMG when HP ≤ 50% (adjacent mult change handled in onUlt) ──
      if (member.eidolon >= 6 && action.type === 'ultimate' && hpPct <= 0.5) {
        member.buffs.dmg_boost += 20;
      }
    },

    onAfterAction: (state, member, action, target) => {
      // ── E2: Remove 1 debuff after Skill ──
      // (Ult debuff removal is handled inside onUlt)
      if (member.eidolon >= 2 && action.type === 'skill') {
        removeOneDebuff(state);
      }

      // ── A2: Restore 20% Max HP when defeating an enemy at ≤ 30% HP ──
      if (target && target.hp <= 0) {
        const original = getArlan(state);
        if (original && original.hp / original.max_hp <= 0.30) {
          const restore = original.max_hp * 0.20;
          original.hp = Math.min(original.max_hp, original.hp + restore);
          state.addLog({
            type: 'event',
            message: `Arlan A2: +20% Max HP restored on kill (HP: ${Math.floor(original.hp)}/${Math.floor(original.max_hp)})`
          });
        }
      }
    },

    onUlt: (state, member) => {
      // Reset energy (not auto-reset since onUlt is defined); ult grants 5 energy
      state.stacks[member.characterId] = 5;

      // ── E2: Remove 1 debuff on Ultimate ──
      if (member.eidolon >= 2) removeOneDebuff(state);

      const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
      if (aliveEnemies.length === 0) return;

      const mainTarget = aliveEnemies[0];
      const mainTargetIdx = state.enemies.indexOf(mainTarget);
      const hpPct = member.max_hp > 0 ? member.hp / member.max_hp : 1;

      // E6: Adjacent multiplier = 320% when HP ≤ 50%, else 160%
      const adjacentMult = (member.eidolon >= 6 && hpPct <= 0.5) ? 3.2 : 1.6;

      let totalUltDmg = 0;

      // ── Main target: 320% ────────────────────────────────────────────────────
      const mainResult = calculateHsrDamage({
        character: member,
        lightcone: member.lightcone,
        enemy: mainTarget,
        ability_multiplier: 3.2,
        scaling_stat_id: ATK_ID
      });
      totalUltDmg += mainResult.expected_dmg;
      mainTarget.hp = Math.max(0, Math.floor(mainTarget.hp - mainResult.expected_dmg));
      state.addLog({ type: 'event', message: `Hit: Main on ${mainTarget.name} -> ${mainResult.expected_dmg.toLocaleString()} DMG (320%)` });
      if ((state as any).applyToughnessDamage) (state as any).applyToughnessDamage(mainTarget, 20, false);

      // ── Adjacent targets ─────────────────────────────────────────────────────
      const leftEnemy  = mainTargetIdx > 0 ? state.enemies[mainTargetIdx - 1] : null;
      const rightEnemy = mainTargetIdx < state.enemies.length - 1 ? state.enemies[mainTargetIdx + 1] : null;
      const adjacentTargets: SimEnemy[] = [];
      if (leftEnemy  && leftEnemy.hp  > 0) adjacentTargets.push(leftEnemy);
      if (rightEnemy && rightEnemy.hp > 0) adjacentTargets.push(rightEnemy);

      adjacentTargets.forEach(adj => {
        const adjResult = calculateHsrDamage({
          character: member,
          lightcone: member.lightcone,
          enemy: adj,
          ability_multiplier: adjacentMult,
          scaling_stat_id: ATK_ID
        });
        totalUltDmg += adjResult.expected_dmg;
        adj.hp = Math.max(0, Math.floor(adj.hp - adjResult.expected_dmg));
        state.addLog({
          type: 'event',
          message: `Hit: Adjacent on ${adj.name} -> ${adjResult.expected_dmg.toLocaleString()} DMG (${(adjacentMult * 100).toFixed(0)}%)`
        });
        if ((state as any).applyToughnessDamage) (state as any).applyToughnessDamage(adj, 20, false);
      });

      state.totalDamage += totalUltDmg;

      // ── A2: Restore HP if any target died while Arlan HP ≤ 30% ──
      const original = getArlan(state);
      const anyKill = mainTarget.hp <= 0 || adjacentTargets.some(t => t.hp <= 0);
      if (original && anyKill && original.hp / original.max_hp <= 0.30) {
        const restore = original.max_hp * 0.20;
        original.hp = Math.min(original.max_hp, original.hp + restore);
        state.addLog({
          type: 'event',
          message: `Arlan A2: +20% Max HP restored on kill (HP: ${Math.floor(original.hp)}/${Math.floor(original.max_hp)})`
        });
      }

      state.addLog({
        type: 'event',
        message: `Frenzied Punishment: ${totalUltDmg.toLocaleString()} total DMG` +
          (adjacentTargets.length > 0 ? ` (${adjacentTargets.length} adjacent @ ${(adjacentMult * 100).toFixed(0)}%)` : '')
      });
    }
  },

  special_modifiers: {
    energy_type: "ENERGY",
    energy_cost: 110,
    stat_boosts: (state: any) => ({
      atk_percent: 28, // Minor trace: ATK +28%
      // HP +10% is not tracked in combat buffs struct
      // Effect RES +18% is not tracked in combat buffs struct (A4 adds its own 50% on top)
    }),
    eidolon_level_boosts: (eidolon: number) => ({
      ...(eidolon >= 3 ? { skill: 2, basic: 1 } : {}),
      ...(eidolon >= 5 ? { ultimate: 2, talent: 2 } : {})
    })
  }
};
