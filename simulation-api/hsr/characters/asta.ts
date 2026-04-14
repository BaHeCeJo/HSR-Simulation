/**
 * @character Asta
 * @role Support / Harmony (Fire)
 * @core_mechanics
 *   Charging stacks (max 5): +14% ATK per stack to ALL allies.
 *   Gained during basic/skill hits (1 per unique enemy + 1 if Fire Weakness).
 *   From 2nd turn onward, -3 stacks per turn start (-2 at E6).
 *   Ultimate: +50 SPD to all allies for 2 turns (tracked per ally via onAllyAction).
 *
 * Implemented mechanics:
 *   Talent    ECharging stacks ↁEall allies +14% ATK/stack (dynamic, updates on gain/loss)
 *   A2        EBasic ATK 80% chance to Burn for 3 turns (DoT = 50% of basic hit DMG)
 *   A4        EAll Fire-element allies +18% Fire DMG at battle start
 *   A6        EAsta DEF +6%/Charging stack (minor, not tracked in combat buffs)
 *   E1        ESkill fires 1 extra bounce (5 ↁE6 total hits)
 *   E2        EAfter Ult, skip next Charging stack reduction
 *   E3        ESkill +2, Talent +2 (via eidolon_level_boosts)
 *   E4        E+15% ERR when 2+ Charging stacks (~+3 on basic, ~+1 on skill)
 *   E5        EUltimate +2, Basic +1 (via eidolon_level_boosts)
 *   E6        ECharging stack reduction per turn: -3 ↁE-2
 *
 * Energy: Basic +20 (auto), Skill +6 (corrected from simulator +30 in onBeforeAction), Ult +5 (onUlt)
 * SPD buff expiry: decremented via onAllyAction (other allies) and onAfterAction (Asta herself).
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

const ATK_ID    = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const CHAR_SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';

const ASTA_ID     = 'a8b9c0d1-e2f3-4567-89ab-cdef01234567';

// State keys
const CHARGING_KEY  = 'asta_charging';
const E2_SKIP_KEY   = 'asta_e2_skip_reduction';
const TURN_KEY      = 'asta_turn_count';
const MAX_CHARGING  = 5;

// ── Helpers ───────────────────────────────────────────────────────────────────

function getAsta(state: SimState): TeamMember | undefined {
  return state.team.find(m => m.characterId === ASTA_ID);
}

/**
 * Update Charging stacks and propagate the ATK buff delta to ALL team members.
 * Handles capping at MAX_CHARGING and clamping at 0.
 */
function setCharging(state: SimState, newStacks: number): void {
  const old = state.stacks[CHARGING_KEY] || 0;
  const capped = Math.min(Math.max(0, newStacks), MAX_CHARGING);
  const diff = (capped - old) * 14; // 14% ATK per stack
  if (diff !== 0) {
    state.team.forEach(m => {
      if (!m.isDowned) m.buffs.atk_percent += diff;
    });
  }
  state.stacks[CHARGING_KEY] = capped;
}

/**
 * Decrement SPD buff remaining turns for a single ally.
 * Reverts +50 SPD when the buff expires.
 */
function decrementSpdBuff(state: SimState, ally: TeamMember): void {
  const key = 'asta_spd_remaining_' + ally.characterId;
  if (!state.stacks[key]) return;
  state.stacks[key]--;
  if (state.stacks[key] <= 0) {
    ally.base_stats[CHAR_SPD_ID] = (ally.base_stats[CHAR_SPD_ID] || 100) - 50;
    state.stacks[key] = 0;
    state.addLog({
      type: 'event',
      message: `Asta: SPD +50 expired for ${ally.name} (SPD now ${(ally.base_stats[CHAR_SPD_ID]).toFixed(0)})`
    });
  }
}

// ── Kit ───────────────────────────────────────────────────────────────────────

export const Asta: CharacterKit = {
  id: ASTA_ID,
  name: "Asta",
  path: "Harmony",
  element: "Fire",
  slot_names: {
    basic:   "Spectrum Beam",
    skill:   "Meteor Storm",
    ultimate:"Astral Blessing",
    talent:  "Astrometry",
  },
  abilities: {
    basic: {
      default_multiplier: 1.0,        // Lv.6: 100% ATK
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 10
    },
    skill: {
      // First hit only; the remaining 4 (or 5 at E1) bounces are fired in onAfterAction
      default_multiplier: 0.5,        // Lv.10: 50% ATK per bounce
      stat_id: ATK_ID,
      targetType: 'Bounce',
      toughness_damage: 10
    },
    ultimate: {
      default_multiplier: 0,          // No direct DMG  Epure support
      stat_id: ATK_ID,
      targetType: 'Support'
    },
    talent: {
      default_multiplier: 0,
      stat_id: ATK_ID
    }
  },

  hooks: {
    // ── Battle Start ───────────────────────────────────────────────────────────
    onBattleStart: (state, member) => {
      state.stacks[CHARGING_KEY] = 0;
      state.stacks[TURN_KEY]     = 0;

      // A4: All Fire-element allies +18% Fire DMG (Asta herself is Fire so she also benefits)
      state.team.forEach(m => {
        if (m.element === 'Fire') {
          m.buffs.dmg_boost += 18;
        }
      });
      state.addLog({ type: 'event', message: `Asta A4: Fire DMG +18% applied to all Fire allies.` });
    },

    // ── Turn Start ─────────────────────────────────────────────────────────────
    onTurnStart: (state, member) => {
      state.stacks[TURN_KEY] = (state.stacks[TURN_KEY] || 0) + 1;

      // From 2nd turn onward: reduce Charging stacks
      if (state.stacks[TURN_KEY] >= 2) {
        if (state.stacks[E2_SKIP_KEY]) {
          // E2: skip reduction this turn
          state.stacks[E2_SKIP_KEY] = 0;
          state.addLog({ type: 'event', message: `Asta E2: Charging reduction skipped this turn.` });
        } else {
          const reduction = member.eidolon >= 6 ? 2 : 3; // E6 reduces by 1 less
          const current = state.stacks[CHARGING_KEY] || 0;
          if (current > 0) {
            const next = Math.max(0, current - reduction);
            setCharging(state, next);
            state.addLog({
              type: 'event',
              message: `Asta Talent: Charging -${reduction} at turn start (${current} ↁE${next}).`
            });
          }
        }
      }

      const stacks = state.stacks[CHARGING_KEY] || 0;
      if (stacks > 0) {
        state.addLog({
          type: 'event',
          message: `Asta Talent: ${stacks} Charging stacks  EAll allies +${stacks * 14}% ATK.`
        });
      }
    },

    // ── Before Action ─────────────────────────────────────────────────────────
    onBeforeAction: (state, member, action) => {
      if (action.type === 'skill') {
        // Simulator will add +30 energy after this action; we need net +6.
        // Pre-subtract 24 so that: (current - 24) + 30 = current + 6.
        state.stacks[ASTA_ID] = (state.stacks[ASTA_ID] || 0) - 24;

        // E4: +15% ERR when 2+ stacks ↁE~+1 extra (6 ÁE0.15 = 0.9 ≁E1)
        if (member.eidolon >= 4 && (state.stacks[CHARGING_KEY] || 0) >= 2) {
          state.stacks[ASTA_ID] += 1;
        }
      } else if (action.type === 'basic') {
        // E4: +15% ERR when 2+ stacks ↁE+3 extra (20 ÁE0.15 = 3)
        if (member.eidolon >= 4 && (state.stacks[CHARGING_KEY] || 0) >= 2) {
          state.stacks[ASTA_ID] = (state.stacks[ASTA_ID] || 0) + 3;
        }
      }
    },

    // ── After Action ──────────────────────────────────────────────────────────
    onAfterAction: (state, member, action, target) => {

      // ── BASIC ATK ──────────────────────────────────────────────────────────
      if (action.type === 'basic' && target) {
        // Talent: +1 Charging for hitting main target, +1 extra if Fire Weakness
        let chargeGain = 1;
        if (target.weaknesses.includes('Fire')) chargeGain++;
        const before = state.stacks[CHARGING_KEY] || 0;
        setCharging(state, before + chargeGain);
        state.addLog({
          type: 'event',
          message: `Asta Talent: +${chargeGain} Charging from Basic (${before} ↁE${state.stacks[CHARGING_KEY]}).`
        });

        // A2: 80% base chance to Burn for 3 turns
        if (Math.random() < 0.80) {
          // DoT value = 50% of basic ATK DMG (recalculated with current member stats)
          const basicResult = calculateHsrDamage({
            character: member,
            lightcone: member.lightcone,
            enemy: target,
            ability_multiplier: 1.0,
            scaling_stat_id: ATK_ID
          });
          const dotDmg = Math.floor(basicResult.expected_dmg * 0.5);
          target.activeDebuffs['asta_burn'] = { duration: 3, value: dotDmg };
          state.addLog({
            type: 'event',
            message: `Asta A2: Burn on ${target.name}  E${dotDmg.toLocaleString()} DoT/turn for 3 turns.`
          });
        }

        // Decrement Asta's own SPD buff after her action
        decrementSpdBuff(state, member);
      }

      // ── SKILL (Bounce) ─────────────────────────────────────────────────────
      if (action.type === 'skill') {
        const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
        if (aliveEnemies.length === 0) {
          decrementSpdBuff(state, member);
          return;
        }

        // Snapshot fire-weakness info BEFORE enemies potentially die in bounces
        const fireWeakMap = new Map<string, boolean>();
        aliveEnemies.forEach(e => fireWeakMap.set(e.instanceId, e.weaknesses.includes('Fire')));

        // Track unique enemies hit (main target was already hit by simulator)
        const hitIds = new Set<string>();
        if (target) hitIds.add(target.instanceId);

        // E1: +1 extra bounce (4 ↁE5 extra hits after the first)
        const extraBounces = 4 + (member.eidolon >= 1 ? 1 : 0);
        let totalBounceDmg = 0;

        for (let i = 0; i < extraBounces; i++) {
          // Re-query alive enemies each bounce in case one died
          const currentAlive = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
          if (currentAlive.length === 0) break;

          const bounceTarget = currentAlive[Math.floor(Math.random() * currentAlive.length)];
          const result = calculateHsrDamage({
            character: member,
            lightcone: member.lightcone,
            enemy: bounceTarget,
            ability_multiplier: 0.5,
            scaling_stat_id: ATK_ID
          });

          bounceTarget.hp = Math.max(0, Math.floor(bounceTarget.hp - result.expected_dmg));
          state.totalDamage  += result.expected_dmg;
          totalBounceDmg     += result.expected_dmg;
          hitIds.add(bounceTarget.instanceId);

          state.addLog({
            type: 'event',
            message: `Bounce ${i + 1}: ${bounceTarget.name} ↁE${result.expected_dmg.toLocaleString()} DMG (HP: ${bounceTarget.hp.toLocaleString()}/${bounceTarget.max_hp.toLocaleString()})`
          });
        }

        if (state.checkEnemies) state.checkEnemies();

        // Talent: Charging stacks from skill (1 per unique enemy hit + 1 if Fire Weakness)
        let chargeGain = 0;
        hitIds.forEach(id => {
          chargeGain++;
          if (fireWeakMap.get(id)) chargeGain++;
        });

        if (chargeGain > 0) {
          const before = state.stacks[CHARGING_KEY] || 0;
          setCharging(state, before + chargeGain);
          state.addLog({
            type: 'event',
            message: `Asta Talent: +${chargeGain} Charging from Skill (${before} ↁE${state.stacks[CHARGING_KEY]}).`
          });
        }

        state.addLog({
          type: 'event',
          message: `Meteor Storm: ${totalBounceDmg.toLocaleString()} total bounce DMG (${extraBounces} extra hits, ${hitIds.size} unique enemies).`
        });

        // Decrement Asta's own SPD buff after her action
        decrementSpdBuff(state, member);
      }
    },

    // ── Ultimate ──────────────────────────────────────────────────────────────
    onUlt: (state, member) => {
      // Manual energy reset: ult costs 120 and grants 5 energy
      state.stacks[ASTA_ID] = 5;

      // E2: skip next Charging reduction on Asta's turn
      if (member.eidolon >= 2) {
        state.stacks[E2_SKIP_KEY] = 1;
        state.addLog({ type: 'event', message: `Asta E2: Next Charging reduction will be skipped.` });
      }

      // Apply SPD +50 to all alive allies for 2 turns
      const aliveTeam = state.team.filter(m => !m.isDowned);
      aliveTeam.forEach(m => {
        m.base_stats[CHAR_SPD_ID] = (m.base_stats[CHAR_SPD_ID] || 100) + 50;
        state.stacks['asta_spd_remaining_' + m.characterId] = 2;
      });

      const names = aliveTeam.map(m => m.name || m.characterId).join(', ');
      state.addLog({
        type: 'event',
        message: `Astral Blessing: All allies [${names}] gain SPD +50 for 2 turns.`
      });
    },

    // ── Enemy Turn: Apply Burn DoT ─────────────────────────────────────────────
    onEnemyTurnStart: (state, member, enemy) => {
      const burn = enemy.activeDebuffs['asta_burn'];
      if (burn?.value) {
        enemy.hp = Math.max(0, enemy.hp - burn.value);
        state.totalDamage += burn.value;
        state.addLog({
          type: 'event',
          message: `Asta A2: Burn DoT ${burn.value.toLocaleString()} on ${enemy.name} (HP: ${enemy.hp.toLocaleString()}/${enemy.max_hp.toLocaleString()}).`
        });
        if (state.checkEnemies) state.checkEnemies();
      }
    },

    // ── Ally Action: Decrement SPD buff for the acting ally ───────────────────
    onAllyAction: (state, member, source, actionType) => {
      decrementSpdBuff(state, source);
    }
  },

  special_modifiers: {
    energy_type: "ENERGY",
    energy_cost: 120,
    stat_boosts: (stats: any) => ({
      dmg_boost: 22.4,  // Minor trace: Fire DMG +22.4% (Asta herself)
      crit_rate:  6.7,  // Minor trace: CRIT Rate +6.7%
      // DEF +22.5%  Enot tracked in combat buffs struct
    }),
    eidolon_level_boosts: (eidolon: number) => ({
      ...(eidolon >= 3 ? { skill: 2, talent: 2 } : {}),
      ...(eidolon >= 5 ? { ultimate: 2, basic: 1 } : {})
    })
  }
};
