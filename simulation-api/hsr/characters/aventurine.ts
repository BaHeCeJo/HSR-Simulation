/**
 * @character Aventurine
 * @role Preservation / Imaginary (Tank + Sub-DPS)
 * @core_mechanics
 *   All damage and shields scale off DEF (not ATK).
 *   `buffs.atk_percent` is repurposed as DEF% for this character.
 *   LC DEF is folded into base_stats[DEF_ID] at onBattleStart for formula compatibility.
 *
 *   Fortified Wager (FW)  Estackable shield on allies, cap = 200% of one skill application.
 *   Blind Bet (BB, 0 E0)  Eaccumulates when:
 *     • A shielded ally is attacked  ↁE +1 BB
 *     • Aventurine himself is attacked (if shielded)  ↁE +1 extra BB
 *     • Ult fired  ↁE +1 E random BB
 *   At 7 BB: auto-fires the talent FUP (7 hits; 10 hits at E4), consuming 7 BB.
 *
 * Implemented:
 *   Basic     E100% DEF, single target. E2: target All-Type RES -12% for 3 turns.
 *   Skill     ENo damage; shields all allies (24% DEF + 320), 3-turn stackable to 200%.
 *   Ultimate  E270% DEF; Unnerved 3 turns (+15% CRIT DMG to allies hitting target);
 *              +1 E random BB; E1: grant all allies skill shield after ult.
 *   Talent    EfireTalentFUP(): 7ÁE5% DEF (10ÁE5% at E4) on random enemies.
 *              E4: +40% DEF for 2 turns before FUP.
 *              A6: after FUP, all allies +7.2% DEF+96 shield; lowest-shield ally gets it twice.
 *   A2  E+2% CRIT Rate per 100 DEF above 1600 (max +48%).
 *   A4  EAll allies receive 100% skill shield at battle start.
 *   A6  EPost-FUP mini-shield (see above).
 *   E1  EShielded allies +20% CRIT DMG (applied globally at battle start).
 *   E2  EBasic ATK: All-Type RES -12% for 3 turns (tracked & reverted per enemy turn).
 *   E4  ETalent FUP: +40% DEF buff for 2 turns; +3 extra hits (7ↁE0).
 *   E6  E+50% DMG per shielded ally (max +150%).
 *
 * Energy: Basic +20 (auto), Skill +30 (auto), Ult +5 (onUlt), FUP +1/hit.
 * Unnerved CRIT DMG and E2 RES debuff are reverted on enemy turn via onEnemyTurnStart.
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

// ─── UUIDs ────────────────────────────────────────────────────────────────────
const DEF_ID     = '73868117-3df2-470d-945a-e389f9f04200'; // Character DEF stat
const DEF_ID_LC  = '52566b38-915c-4220-ab0e-61438225704b'; // Lightcone DEF stat

const AVENTURINE_ID = 'c2d3e4f5-a6b7-4890-c1d2-e3f4a5b6c7d8';

// ─── State keys ──────────────────────────────────────────────────────────────
const BB_KEY          = 'aventurine_bb';
const E4_TURNS_KEY    = 'aventurine_e4_def_turns';
const UNNERVED_ACTIVE = 'aventurine_unnerved_active'; // Flag: global CRIT DMG buff applied

// ─── Helpers ─────────────────────────────────────────────────────────────────

function getAventurine(state: SimState): TeamMember | undefined {
  return state.team.find(m => m.characterId === AVENTURINE_ID);
}

/**
 * Total DEF after DEF% (stored in buffs.atk_percent for this character).
 * LC DEF is pre-folded into base_stats[DEF_ID] at onBattleStart.
 */
function getDef(member: TeamMember): number {
  return (member.base_stats[DEF_ID] || 0) * (1 + member.buffs.atk_percent / 100);
}

/** Skill-level shield value: 24% DEF + 320 (Lv.10). */
function getSkillShield(member: TeamMember): number {
  return Math.floor(0.24 * getDef(member) + 320);
}

/** A6 post-FUP mini-shield: 7.2% DEF + 96. */
function getA6Shield(member: TeamMember): number {
  return Math.floor(0.072 * getDef(member) + 96);
}

/** Apply a shield value to all alive allies, capped at 200% of the current skill shield. */
function applyShieldsToAll(state: SimState, aventurine: TeamMember, shieldValue: number): void {
  const cap = 2 * getSkillShield(aventurine);
  state.team.forEach(m => {
    if (!m.isDowned) {
      m.shield = Math.floor(Math.min(m.shield + shieldValue, cap));
    }
  });
}

/**
 * Add Blind Bet points (capped at 10) and auto-trigger FUP at 7+.
 * `source` is the member whose eidolon level and live buffs we read.
 */
function addBB(state: SimState, amount: number): void {
  state.stacks[BB_KEY] = Math.min(10, (state.stacks[BB_KEY] || 0) + amount);
  if (state.stacks[BB_KEY] >= 7) {
    fireTalentFUP(state);
  }
}

/**
 * Consume 7 BB and fire the talent follow-up attack:
 *   7ÁE5% DEF hits (10 with E4) on random enemies.
 *   E4: +40% DEF boost for 2 turns.
 *   A6: mini-shield to all allies + extra to lowest-shield ally.
 */
function fireTalentFUP(state: SimState): void {
  const av = getAventurine(state);
  if (!av || av.isDowned) return;

  const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
  if (aliveEnemies.length === 0) return;

  state.stacks[BB_KEY] -= 7;

  // E4: +40% DEF for 2 turns
  if (av.eidolon >= 4) {
    av.buffs.atk_percent += 40;
    state.stacks[E4_TURNS_KEY] = 2;
    state.addLog({ type: 'event', message: `Aventurine E4: DEF +40% for 2 turns.` });
  }

  const hits = av.eidolon >= 4 ? 10 : 7;

  // Snapshot attacker stats (include E6 DMG boost)
  const fupMember = { ...av, buffs: { ...av.buffs } };
  if (av.eidolon >= 6) {
    const shieldedCount = state.team.filter(m => !m.isDowned && m.shield > 0).length;
    fupMember.buffs.dmg_boost += Math.min(3, shieldedCount) * 50;
  }

  let totalFUPDmg = 0;

  for (let i = 0; i < hits; i++) {
    const current = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
    if (current.length === 0) break;

    const target = current[Math.floor(Math.random() * current.length)];
    const result = calculateHsrDamage({
      character: fupMember,
      lightcone: av.lightcone,
      enemy: target,
      ability_multiplier: 0.25,
      scaling_stat_id: DEF_ID
    });

    target.hp = Math.max(0, Math.floor(target.hp - result.expected_dmg));
    state.totalDamage += result.expected_dmg;
    totalFUPDmg += result.expected_dmg;

    state.addLog({ type: 'event', message: `FUP Hit ${i + 1}: ${target.name} ↁE${result.expected_dmg.toLocaleString()} DMG` });

    // Toughness: Break 3 per hit
    if ((state as any).applyToughnessDamage) {
      (state as any).applyToughnessDamage(target, 3, false);
    }

    // Energy: +1 per hit
    state.stacks[AVENTURINE_ID] = (state.stacks[AVENTURINE_ID] || 0) + 1;
  }

  if (state.checkEnemies) state.checkEnemies();

  // A6: mini-shield to all + extra to lowest-shield ally
  const a6Val   = getA6Shield(av);
  const shieldCap = 2 * getSkillShield(av);
  state.team.forEach(m => {
    if (!m.isDowned) m.shield = Math.floor(Math.min(m.shield + a6Val, shieldCap));
  });
  const lowestShieldAlly = state.team
    .filter(m => !m.isDowned)
    .reduce((min, m) => (m.shield < min.shield ? m : min), state.team.filter(m => !m.isDowned)[0]);
  if (lowestShieldAlly) {
    lowestShieldAlly.shield = Math.floor(Math.min(lowestShieldAlly.shield + a6Val, shieldCap));
  }

  state.addLog({
    type: 'event',
    message: `Talent FUP: ${hits} hits  E${totalFUPDmg.toLocaleString()} total DMG. A6: +${a6Val} shield to all (${lowestShieldAlly?.name} gets +${a6Val * 2} total as lowest-shield).`
  });
}

// ─── Kit ─────────────────────────────────────────────────────────────────────

export const Aventurine: CharacterKit = {
  id: AVENTURINE_ID,
  name: "Aventurine",
  path: "Preservation",
  element: "Imaginary",
  slot_names: {
    basic:    "Straight Bet",
    skill:    "Cornerstone Deluxe",
    ultimate: "Roulette Shark",
    talent:   "Shot Loaded Right",
  },
  abilities: {
    basic: {
      default_multiplier: 1.0, // Lv.6: 100% DEF
      stat_id: DEF_ID,
      targetType: 'SingleTarget',
      toughness_damage: 10
    },
    skill: {
      // No damage  Epure shield. Nested to avoid falsy `default_multiplier: 0` misparse.
      main: {
        default_multiplier: 0,
        stat_id: DEF_ID,
        targetType: 'Defense'
      }
    },
    ultimate: {
      default_multiplier: 2.7, // Lv.10: 270% DEF (handled in onUlt)
      stat_id: DEF_ID,
      targetType: 'SingleTarget',
      toughness_damage: 30
    },
    talent: {
      default_multiplier: 0.25, // Lv.10: 25% DEF per hit (FUP only  Efired manually)
      stat_id: DEF_ID
    }
  },

  hooks: {
    // ── Battle Start ──────────────────────────────────────────────────────────
    onBattleStart: (state, member) => {
      // Fold LC DEF into character base_stats so the formula picks it up via DEF_ID
      const lcDef = member.lightcone.base_stats[DEF_ID_LC] || 0;
      if (lcDef > 0) {
        member.base_stats[DEF_ID] = (member.base_stats[DEF_ID] || 0) + lcDef;
        state.addLog({ type: 'event', message: `Aventurine: Folded LC DEF ${lcDef} into base DEF.` });
      }

      state.stacks[BB_KEY] = 0;

      // A2: +2% CRIT Rate per 100 DEF above 1600 (max +48%)
      const totalDef = getDef(member);
      const excessDef = Math.max(0, totalDef - 1600);
      const crGain   = Math.min(48, Math.floor(excessDef / 100) * 2);
      if (crGain > 0) {
        member.buffs.crit_rate += crGain;
        state.addLog({ type: 'event', message: `Aventurine A2: ${totalDef.toFixed(0)} DEF ↁE+${crGain}% CRIT Rate.` });
      }

      // A4: All allies receive 100% skill-shield at battle start
      const shieldVal = getSkillShield(member);
      applyShieldsToAll(state, member, shieldVal);
      state.addLog({ type: 'event', message: `Aventurine A4: All allies +${shieldVal} Fortified Wager (battle start).` });

      // E1: Shielded allies gain +20% CRIT DMG (applied globally)
      if (member.eidolon >= 1) {
        state.team.forEach(m => { m.buffs.crit_dmg += 20; });
        state.addLog({ type: 'event', message: `Aventurine E1: All shielded allies +20% CRIT DMG.` });
      }
    },

    // ── Turn Start ────────────────────────────────────────────────────────────
    onTurnStart: (state, member) => {
      // E4: Decrement DEF boost counter
      if (member.eidolon >= 4 && (state.stacks[E4_TURNS_KEY] || 0) > 0) {
        state.stacks[E4_TURNS_KEY]--;
        if (state.stacks[E4_TURNS_KEY] <= 0) {
          member.buffs.atk_percent -= 40;
          state.addLog({ type: 'event', message: `Aventurine E4: DEF +40% expired.` });
        }
      }
    },

    // ── Before Action ─────────────────────────────────────────────────────────
    onBeforeAction: (state, member, action) => {
      // E6: +50% DMG per shielded ally (max +150%) for basic ATK
      if (member.eidolon >= 6 && action.type === 'basic') {
        const shieldedCount = state.team.filter(m => !m.isDowned && m.shield > 0).length;
        const e6Boost = Math.min(3, shieldedCount) * 50;
        if (e6Boost > 0) {
          member.buffs.dmg_boost += e6Boost;
        }
      }
    },

    // ── After Action ──────────────────────────────────────────────────────────
    onAfterAction: (state, member, action, target) => {
      // ── BASIC ATK ──────────────────────────────────────────────────────────
      if (action.type === 'basic' && target) {
        // E2: All-Type RES -12% for 3 turns
        if (member.eidolon >= 2) {
          const wasActive = !!target.activeDebuffs['aventurine_e2_res'];
          if (!wasActive) {
            // Apply RES reduction (additive)
            target.resistance = Math.max(0, target.resistance - 0.12);
            Object.keys(target.elemental_res).forEach(el => {
              target.elemental_res[el] = Math.max(0, target.elemental_res[el] - 0.12);
            });
            state.addLog({ type: 'event', message: `Aventurine E2: ${target.name} All-Type RES -12% for 3 turns.` });
          } else {
            state.addLog({ type: 'event', message: `Aventurine E2: RES debuff refreshed on ${target.name}.` });
          }
          target.activeDebuffs['aventurine_e2_res'] = { duration: 3, value: 12, stat: 'All-Type RES' };
          state.stacks['aventurine_e2_turns_' + target.instanceId] = 3;
        }
      }

      // ── SKILL (Shield application) ─────────────────────────────────────────
      if (action.type === 'skill') {
        const av = getAventurine(state);
        if (!av) return;
        const shieldVal = getSkillShield(av);
        applyShieldsToAll(state, av, shieldVal);
        state.addLog({
          type: 'event',
          message: `Cornerstone Deluxe: All allies +${shieldVal} Fortified Wager (cap: ${2 * shieldVal}).`
        });
      }
    },

    // ── Ultimate ──────────────────────────────────────────────────────────────
    onUlt: (state, member) => {
      const av = getAventurine(state);
      if (!av) return;

      // Energy reset: ult costs 110, grants 5
      state.stacks[AVENTURINE_ID] = 5;

      const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
      if (aliveEnemies.length === 0) return;
      const target = aliveEnemies[0];

      // Apply Unnerved debuff (3 turns) ↁEglobal +15% CRIT DMG to all allies
      if (!state.stacks[UNNERVED_ACTIVE]) {
        state.team.forEach(m => { m.buffs.crit_dmg += 15; });
        state.stacks[UNNERVED_ACTIVE] = 1;
        state.addLog({ type: 'event', message: `Aventurine Ult: Unnerved on ${target.name}  Eallies +15% CRIT DMG.` });
      } else {
        state.addLog({ type: 'event', message: `Aventurine Ult: Unnerved refreshed on ${target.name}.` });
      }
      target.activeDebuffs['aventurine_unnerved'] = { duration: 3, stat: 'Unnerved', value: 15 };
      state.stacks['aventurine_unnerved_turns_' + target.instanceId] = 3;

      // Random BB gain: 1 E
      const bbGain = Math.floor(Math.random() * 7) + 1;
      state.addLog({ type: 'event', message: `Aventurine Ult: +${bbGain} Blind Bet (1 E range).` });

      // Deal 270% DEF damage (manual; includes E6 DMG boost)
      const ultMember = { ...av, buffs: { ...av.buffs } };
      if (av.eidolon >= 6) {
        const shieldedCount = state.team.filter(m => !m.isDowned && m.shield > 0).length;
        ultMember.buffs.dmg_boost += Math.min(3, shieldedCount) * 50;
      }

      const result = calculateHsrDamage({
        character: ultMember,
        lightcone: av.lightcone,
        enemy: target,
        ability_multiplier: 2.7,
        scaling_stat_id: DEF_ID
      });
      target.hp = Math.max(0, Math.floor(target.hp - result.expected_dmg));
      state.totalDamage += result.expected_dmg;
      state.addLog({
        type: 'event',
        message: `Roulette Shark: ${target.name} ↁE${result.expected_dmg.toLocaleString()} DMG (270% DEF)`
      });
      if ((state as any).applyToughnessDamage) (state as any).applyToughnessDamage(target, 30, false);
      if (state.checkEnemies) state.checkEnemies();

      // E1: grant all allies skill shield after ult
      if (av.eidolon >= 1) {
        const shieldVal = getSkillShield(av);
        applyShieldsToAll(state, av, shieldVal);
        state.addLog({ type: 'event', message: `Aventurine E1: All allies +${shieldVal} after ult.` });
      }

      // Add BB (and potentially trigger FUP)
      addBB(state, bbGain);
    },

    // ── Enemy Action: Accumulate Blind Bet ────────────────────────────────────
    onEnemyAction: (state, member, enemy) => {
      const av = getAventurine(state);
      if (!av || av.isDowned) return;

      // +1 BB when any shielded non-Aventurine ally is attacked
      const anyShieldedAlly = state.team.some(
        m => !m.isDowned && m.shield > 0 && m.characterId !== AVENTURINE_ID
      );
      if (anyShieldedAlly) {
        addBB(state, 1);
        state.addLog({
          type: 'event',
          message: `Aventurine Talent: +1 BB from shielded ally attacked (${state.stacks[BB_KEY]}/10).`
        });
      }

      // +1 BB when Aventurine himself is attacked (if shielded)
      if (av.shield > 0) {
        addBB(state, 1);
        state.addLog({
          type: 'event',
          message: `Aventurine Talent: +1 BB (Aventurine attacked while shielded) (${state.stacks[BB_KEY]}/10).`
        });
      }
    },

    // ── Enemy Turn Start: Debuff duration tracking ────────────────────────────
    onEnemyTurnStart: (state, member, enemy) => {
      // Dedup: only process once per enemy-turn AV (multiple characters may have this hook)
      const dedupKey = 'aventurine_ets_av_' + enemy.instanceId;
      if (state.stacks[dedupKey] === state.currentAV) return;
      state.stacks[dedupKey] = state.currentAV;

      // ── Unnerved duration ───────────────────────────────────────────────────
      const unnervedTurnsKey = 'aventurine_unnerved_turns_' + enemy.instanceId;
      if (enemy.activeDebuffs['aventurine_unnerved']) {
        state.stacks[unnervedTurnsKey] = (state.stacks[unnervedTurnsKey] || 0) - 1;
        if (state.stacks[unnervedTurnsKey] <= 0) {
          delete enemy.activeDebuffs['aventurine_unnerved'];
          if (state.stacks[UNNERVED_ACTIVE]) {
            state.team.forEach(m => { m.buffs.crit_dmg -= 15; });
            state.stacks[UNNERVED_ACTIVE] = 0;
            state.addLog({
              type: 'event',
              message: `Aventurine: Unnerved expired on ${enemy.name}. Allies CRIT DMG -15% reverted.`
            });
          }
        }
      }

      // ── E2 RES debuff duration ──────────────────────────────────────────────
      const e2TurnsKey = 'aventurine_e2_turns_' + enemy.instanceId;
      if (enemy.activeDebuffs['aventurine_e2_res']) {
        state.stacks[e2TurnsKey] = (state.stacks[e2TurnsKey] || 0) - 1;
        if (state.stacks[e2TurnsKey] <= 0) {
          delete enemy.activeDebuffs['aventurine_e2_res'];
          enemy.resistance = Math.min(1, enemy.resistance + 0.12);
          Object.keys(enemy.elemental_res).forEach(el => {
            enemy.elemental_res[el] = Math.min(1, enemy.elemental_res[el] + 0.12);
          });
          state.addLog({
            type: 'event',
            message: `Aventurine E2: RES debuff expired on ${enemy.name}.`
          });
        }
      }
    }
  },

  special_modifiers: {
    energy_type: "ENERGY",
    energy_cost: 110,
    stat_boosts: (stats: any) => ({
      atk_percent: 35,  // Minor trace: DEF +35% (repurposed as DEF% scaling)
      dmg_boost:  14.4, // Minor trace: Imaginary DMG +14.4%
      // Effect RES +10%  Enot tracked in combat buffs struct
    }),
    eidolon_level_boosts: (eidolon: number) => ({
      ...(eidolon >= 3 ? { ultimate: 2, basic: 1 } : {}),
      ...(eidolon >= 5 ? { skill: 2, talent: 2 } : {})
    })
  }
};
