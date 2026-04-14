/**
 * @character Ashveil
 * @role Support / Follow-Up DPS (Hunt / Lightning)
 * @core_mechanic "Bait" marks one enemy: all enemies' DEF ∁E0%, ally attacks on Bait trigger Ashveil's Follow-Up ATK (costs 1 Charge).
 *   Gluttony stacks amplify Follow-Up DMG (A4) and unlock Gluttony-consumption hits inside the Ult chain (E6 DMG).
 * @skill_priority Ultimate > Skill (on Bait for SP refund) > Basic
 * @eidolon_milestones E1 (global vuln 24ↁE6%), E2 (Gluttony cap 18 + refund), E4 (ATK boost on Ult), E6 (RES∁E0% + Gluttony DMG).
 *
 * Global field effects (applied at battle start / Bait establishment):
 *   A6    EAll allies +40% CRIT DMG; Ashveil's follow-ups get additional +80% CRIT DMG
 *   E1    EAll enemies +24% DMG taken; +36% when HP ≤ 50%
 *   Bait  EAll allies +40 def_ignore (once Bait is on field)
 *   E6    EAll enemies ∁E0% All-Type RES (once Bait is on field, E6 only)
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

const ATK_ID      = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const ASHVEIL_ID  = 'f1e2d3c4-b5a6-4789-a0bc-de1f2a3b4c5d';

// ── State keys ─────────────────────────────────────────────────────────────────
const CHARGE_KEY        = 'ashveil_charge';
const GLUTTONY_KEY      = 'ashveil_gluttony';
const GLUTTONY_TOTAL    = 'ashveil_gluttony_total'; // Lifetime total (E6 DMG, capped 30)
const BAIT_KEY          = 'ashveil_bait_target';    // instanceId of Bait enemy
const BAIT_ACTIVE       = 'ashveil_bait_active';
const E4_TURNS          = 'ashveil_e4_turns';
const BAIT_DEF_APPLIED  = 'ashveil_bait_def_applied';
const E6_RES_APPLIED    = 'ashveil_e6_res_applied';

// ── Helpers ───────────────────────────────────────────────────────────────────

function getCharge(state: SimState): number {
  return state.stacks[CHARGE_KEY] || 0;
}

function getGluttony(state: SimState): number {
  return state.stacks[GLUTTONY_KEY] || 0;
}

function getMaxGluttony(eidolon: number): number {
  return eidolon >= 2 ? 18 : 12;
}

/** Add Gluttony stacks (capped), also increments lifetime total for E6. */
function addGluttony(state: SimState, member: TeamMember, amount: number): void {
  const max = getMaxGluttony(member.eidolon);
  const before = getGluttony(state);
  const after = Math.min(before + amount, max);
  state.stacks[GLUTTONY_KEY] = after;
  // Track lifetime total (E6 DMG scales up to 30 stacks of 4%)
  const gained = after - before;
  if (gained > 0) {
    state.stacks[GLUTTONY_TOTAL] = Math.min((state.stacks[GLUTTONY_TOTAL] || 0) + gained, 30);
  }
}

function getBaitTarget(state: SimState): SimEnemy | null {
  const id = state.stacks[BAIT_KEY];
  if (!id || !state.stacks[BAIT_ACTIVE]) return null;
  return state.enemies.find((e): e is SimEnemy => !!e && e.instanceId === String(id) && e.hp > 0) ?? null;
}

/** Move Bait to the alive enemy with the lowest HP (auto-tracking). */
function moveBaitToLowestHP(state: SimState, member: TeamMember): SimEnemy | null {
  const alive = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
  if (alive.length === 0) return null;
  const target = alive.reduce((a, b) => a.hp < b.hp ? a : b);
  applyBait(state, member, target);
  return target;
}

/** Apply/move the Bait to a target. On first Bait: apply DEF reduction + E6 RES. */
function applyBait(state: SimState, member: TeamMember, target: SimEnemy): void {
  // Remove Bait from previous target
  const prevId = state.stacks[BAIT_KEY];
  if (prevId && String(prevId) !== target.instanceId) {
    const prev = state.enemies.find((e): e is SimEnemy => !!e && e.instanceId === String(prevId));
    if (prev) {
      delete prev.activeDebuffs['bait'];
    }
  }

  // Apply Bait to new target
  target.activeDebuffs['bait'] = { duration: 9999, stat: 'Bait' };
  state.stacks[BAIT_KEY] = target.instanceId as any;
  state.stacks[BAIT_ACTIVE] = 1;

  state.addLog({ type: 'event', message: `Bait applied to ${target.name}.` });

  // ── On first Bait establishment: global DEF reduction for all allies ──────
  if (!state.stacks[BAIT_DEF_APPLIED]) {
    state.stacks[BAIT_DEF_APPLIED] = 1;
    state.team.forEach(m => { m.buffs.def_ignore += 40; });
    state.addLog({ type: 'event', message: `Bait: All allies gain +40 DEF Ignore.` });
  }

  // ── E6: All enemies ∁E0% All-Type RES once Bait is established ───────────
  if (member.eidolon >= 6 && !state.stacks[E6_RES_APPLIED]) {
    state.stacks[E6_RES_APPLIED] = 1;
    state.enemies.forEach(e => {
      if (!e) return;
      Object.keys(e.elemental_res).forEach(el => {
        e.elemental_res[el] = Math.max(-1, e.elemental_res[el] - 0.20);
      });
      e.resistance = Math.max(-1, e.resistance - 0.20);
    });
    state.addLog({ type: 'event', message: `Ashveil E6: All enemies ∁E0% All-Type RES (Bait established).` });
  }
}

/** Update E1 vulnerability on all alive enemies based on current HP. */
function updateE1Vulnerability(state: SimState, member: TeamMember): void {
  if (member.eidolon < 1) return;
  state.enemies.forEach(e => {
    if (!e || e.hp <= 0) return;
    const key = `ashveil_e1_${e.instanceId}`;
    const current = state.stacks[key] || 0;
    const next = e.hp / e.max_hp <= 0.5 ? 36 : 24;
    e.vulnerability += (next - current);
    state.stacks[key] = next;
  });
}

/**
 * Core Follow-Up ATK calculation.
 * @param consumeCharge  true for talent-triggered FUPs; false for enhanced Ult FUP.
 * @param isEnhancedChain  if true, don't gain Gluttony from the follow-up itself (Gluttony consumed instead).
 * Returns true if the target was killed.
 */
function fireFUP(state: SimState, ashveil: TeamMember, target: SimEnemy, consumeCharge: boolean): boolean {
  const fuBuffs = { ...ashveil.buffs };
  const fuMember = { ...ashveil, buffs: fuBuffs };

  // E4: ATK boost if active
  if ((state.stacks[E4_TURNS] || 0) > 0) {
    fuMember.buffs.atk_percent += 40;
  }

  // A4: Follow-Up DMG +80%, +10% per current Gluttony stack
  fuMember.buffs.dmg_boost += 80 + getGluttony(state) * 10;

  // A6: Additional +80% CRIT DMG for Follow-Up ATK (on top of the +40% applied at battle start)
  fuMember.buffs.crit_dmg += 80;

  // E6: DMG boost from lifetime Gluttony gained (4% per stack, cap 30)
  if (ashveil.eidolon >= 6) {
    fuMember.buffs.dmg_boost += (state.stacks[GLUTTONY_TOTAL] || 0) * 4;
  }

  const result = calculateHsrDamage({
    character: fuMember,
    lightcone: ashveil.lightcone,
    enemy: target,
    ability_multiplier: 2.0, // Lv.10: 200% ATK
    scaling_stat_id: ATK_ID
  });

  state.totalDamage += result.expected_dmg;
  target.hp = Math.max(0, Math.floor(target.hp - result.expected_dmg));

  if ((state as any).applyToughnessDamage) {
    (state as any).applyToughnessDamage(target, 5, false); // break: 5
  }

  // Energy: +8 per talent trigger
  state.stacks[ASHVEIL_ID] = (state.stacks[ASHVEIL_ID] || 0) + 8;

  if (consumeCharge) {
    state.stacks[CHARGE_KEY] = Math.max(0, getCharge(state) - 1);
  }

  state.addLog({
    type: 'event',
    message: `Ashveil Follow-Up ATK on ${target.name} -> ${result.expected_dmg.toLocaleString()} DMG ` +
             `(Charge: ${getCharge(state)}, Gluttony: ${getGluttony(state)})`
  });

  return target.hp <= 0;
}

/**
 * Enhanced Follow-Up ATK triggered by Ultimate.
 * Does NOT consume Charge.
 * Loops while Gluttony ≥ 4: consume 4 ↁEdeal 200%, possibly carrying to a new Bait on kill.
 */
function launchEnhancedFUP(state: SimState, ashveil: TeamMember): void {
  let baitTarget = getBaitTarget(state);
  if (!baitTarget) baitTarget = moveBaitToLowestHP(state, ashveil);
  if (!baitTarget) return;

  // Base 200% hit (doesn't consume Charge)
  const killed = fireFUP(state, ashveil, baitTarget, false);

  // A2: +1 Gluttony per kill during FUP
  if (killed) {
    addGluttony(state, ashveil, 1);
    baitTarget = moveBaitToLowestHP(state, ashveil);
    if (!baitTarget) return;
  }

  // Gluttony consumption loop: consume 4 ↁEdeal 200%, repeat
  let gluttonyConsumed = 0;
  while (getGluttony(state) >= 4) {
    let currentTarget = getBaitTarget(state);
    if (!currentTarget || currentTarget.hp <= 0) {
      currentTarget = moveBaitToLowestHP(state, ashveil);
      if (!currentTarget) break;
    }

    state.stacks[GLUTTONY_KEY] -= 4;
    gluttonyConsumed += 4;

    const chainKilled = fireFUP(state, ashveil, currentTarget, false);
    if (chainKilled) {
      addGluttony(state, ashveil, 1); // A2: kill bonus
      const next = moveBaitToLowestHP(state, ashveil);
      if (!next) break;
    }
  }

  // E2: Refund 35% of Gluttony consumed in the chain (floored)
  if (ashveil.eidolon >= 2 && gluttonyConsumed > 0) {
    const refund = Math.floor(gluttonyConsumed * 0.35);
    if (refund > 0) {
      addGluttony(state, ashveil, refund);
      state.addLog({ type: 'event', message: `Ashveil E2: Refunded ${refund} Gluttony (35% of ${gluttonyConsumed} consumed). Total: ${getGluttony(state)}` });
    }
  }
}

// ── Kit definition ────────────────────────────────────────────────────────────

export const Ashveil: CharacterKit = {
  id: ASHVEIL_ID,
  name: "Ashveil",
  path: "Hunt",
  element: "Lightning",
  slot_names: {
    basic: "Talons: Inculcate Decorum",
    skill: "Flog: Smite Evil",
    ultimate: "Banquet: Insatiable Appetite",
    talent: "Rancor: Enmity Reprisal",
  },
  abilities: {
    basic: {
      default_multiplier: 1.0, // Lv.6
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 10
    },
    skill: {
      // Multiplier starts at 2.0; boosted to 3.0 in onBeforeAction if target is already Bait
      default_multiplier: 2.0, // Lv.10 base
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 20
    },
    ultimate: {
      default_multiplier: 4.0, // Lv.10: 400% ATK (enhanced FUP handled in onUlt)
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 30
    },
    talent: {
      default_multiplier: 2.0, // Lv.10: 200% ATK (handled via onAllyAction)
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 5
    }
  },
  hooks: {
    onBattleStart: (state, member) => {
      // Initialise state
      state.stacks[CHARGE_KEY]       = 2;
      state.stacks[GLUTTONY_KEY]     = 0;
      state.stacks[GLUTTONY_TOTAL]   = 0;
      state.stacks[BAIT_KEY]         = '' as any;
      state.stacks[BAIT_ACTIVE]      = 0;
      state.stacks[E4_TURNS]         = 0;
      state.stacks[BAIT_DEF_APPLIED] = 0;
      state.stacks[E6_RES_APPLIED]   = 0;

      // A6: All allies +40% CRIT DMG (Ashveil is on the field)
      state.team.forEach(m => { m.buffs.crit_dmg += 40; });
      state.addLog({ type: 'event', message: `Ashveil A6: All allies +40% CRIT DMG.` });

      // E1: Apply initial vulnerability (+24%) to all enemies
      if (member.eidolon >= 1) {
        state.enemies.forEach(e => {
          if (!e) return;
          e.vulnerability += 24;
          state.stacks[`ashveil_e1_${e.instanceId}`] = 24;
        });
        state.addLog({ type: 'event', message: `Ashveil E1: All enemies +24% DMG taken.` });
      }
    },

    onTurnStart: (state, member) => {
      // E4: Decrement ATK buff counter
      if ((state.stacks[E4_TURNS] || 0) > 0) {
        state.stacks[E4_TURNS]--;
      }

      // E1: Refresh vulnerability for enemies that crossed the 50% HP threshold
      updateE1Vulnerability(state, member);
    },

    onBeforeAction: (state, member, action, target) => {
      // E4: Apply ATK boost if active
      if ((state.stacks[E4_TURNS] || 0) > 0) {
        member.buffs.atk_percent += 40;
      }

      // E6: DMG boost from lifetime Gluttony gained
      if (member.eidolon >= 6) {
        member.buffs.dmg_boost += (state.stacks[GLUTTONY_TOTAL] || 0) * 4;
      }

      // Skill: If target is already Bait ↁE200% + 100% = 300%; flag SP recovery
      if (action.type === 'skill' && target) {
        const baitId = state.stacks[BAIT_KEY];
        if (baitId && target.instanceId === String(baitId) && state.stacks[BAIT_ACTIVE]) {
          action.multiplier = 3.0; // 200% base + 100% bonus for hitting existing Bait
          state.stacks['ashveil_skill_hit_bait'] = 1;
          state.addLog({ type: 'event', message: `Skill hits existing Bait ↁE300% total (SP refund pending).` });
        }
      }
    },

    onAfterAction: (state, member, action, target) => {
      if (action.type === 'skill' && target) {
        // Apply / move Bait to this target
        applyBait(state, member, target);

        // SP refund if we hit an existing Bait (flag set in onBeforeAction)
        if (state.stacks['ashveil_skill_hit_bait']) {
          state.stacks['ashveil_skill_hit_bait'] = 0;
          state.skillPoints = Math.min(5, (state.skillPoints || 0) + 1);
          state.addLog({ type: 'event', message: `Ashveil Skill (Bait hit): Recovered 1 Skill Point.` });
        }

        // A2: Skill ↁE+1 Gluttony
        addGluttony(state, member, 1);
        state.addLog({ type: 'event', message: `Ashveil A2 (Skill): +1 Gluttony (Total: ${getGluttony(state)})` });

        // E1: Update vulnerability after Bait is established / target HP may have changed
        updateE1Vulnerability(state, member);
      }
    },

    onUlt: (state, member) => {
      // Reset energy; ult grants 5 energy back
      state.stacks[ASHVEIL_ID] = 5;

      const target = state.enemies.find((e): e is SimEnemy => e !== null && e.hp > 0);
      if (!target) return;

      // Apply / move Bait to this target
      applyBait(state, member, target);

      // E4: +40% ATK for 3 turns (set counter; boosts current ult via onBeforeAction)
      if (member.eidolon >= 4) {
        state.stacks[E4_TURNS] = 3;
        member.buffs.atk_percent += 40; // Active for this ult's damage too
      }

      // 400% ATK main hit
      const mainResult = calculateHsrDamage({
        character: member,
        lightcone: member.lightcone,
        enemy: target,
        ability_multiplier: 4.0,
        scaling_stat_id: ATK_ID
      });
      state.totalDamage += mainResult.expected_dmg;
      target.hp = Math.max(0, Math.floor(target.hp - mainResult.expected_dmg));
      state.addLog({ type: 'event', message: `Hit: Ult 400% on ${target.name} -> ${mainResult.expected_dmg.toLocaleString()} DMG` });
      if ((state as any).applyToughnessDamage) (state as any).applyToughnessDamage(target, 30, false);

      // Grant 3 Charge (capped at max 3)
      state.stacks[CHARGE_KEY] = 3;

      // A2: Ult ↁE+2 Gluttony
      addGluttony(state, member, 2);
      state.addLog({ type: 'event', message: `Ashveil A2 (Ult): +2 Gluttony (Total: ${getGluttony(state)}), Charge restored to 3.` });

      // E1: Refresh vulnerability after ult hit (target HP may have changed)
      updateE1Vulnerability(state, member);

      // Enhanced Follow-Up ATK chain
      launchEnhancedFUP(state, member);

      state.addLog({ type: 'event', message: `Post-Ult: Charge=${getCharge(state)}, Gluttony=${getGluttony(state)}, Energy=${state.stacks[ASHVEIL_ID]}` });
    },

    onAllyAction: (state, member, source, actionType, target) => {
      // member = Ashveil, source = the ally who just acted
      // Trigger Talent Follow-Up if Bait exists and Ashveil has Charge
      if (getCharge(state) <= 0) return;

      let baitTarget = getBaitTarget(state);

      // No Bait? Auto-apply to lowest HP enemy (A2-adjacent passive)
      if (!baitTarget) {
        baitTarget = moveBaitToLowestHP(state, member);
        if (!baitTarget) return;
      }

      // Talent fires when ally attacks the Bait. For AoE/Blast actions the Bait is always hit.
      // For SingleTarget, check that the target is the Bait (or assume it's the same enemy in single-target scenarios).
      const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
      const targetIsBait = !target || target.instanceId === baitTarget.instanceId;
      const isAoE = aliveEnemies.length > 1 && (actionType === 'skill' || actionType === 'ultimate');
      if (!targetIsBait && !isAoE) return;

      // Fire talent Follow-Up ATK (consumes 1 Charge)
      const killed = fireFUP(state, member, baitTarget, true);

      // Gain 2 Gluttony from the follow-up
      addGluttony(state, member, 2);
      state.addLog({ type: 'event', message: `Ashveil Talent: +2 Gluttony (Total: ${getGluttony(state)})` });

      // A2: +1 Gluttony if kill during follow-up
      if (killed) {
        addGluttony(state, member, 1);
        state.addLog({ type: 'event', message: `Ashveil A2 (FUP kill): +1 Gluttony (Total: ${getGluttony(state)})` });
        moveBaitToLowestHP(state, member);
      }

      // E1: Refresh vulnerability
      updateE1Vulnerability(state, member);
    }
  },

  special_modifiers: {
    energy_type: "ENERGY",
    energy_cost: 150,
    stat_boosts: (state: any) => ({
      atk_percent: 10,   // Minor trace: ATK +10%
      dmg_boost: 14.4,   // Minor trace: Lightning DMG +14.4%
      crit_dmg: 37.3,    // Minor trace: CRIT DMG +37.3%
    }),
    eidolon_level_boosts: (eidolon: number) => ({
      ...(eidolon >= 3 ? { ultimate: 2, basic: 1 } : {}),
      ...(eidolon >= 5 ? { skill: 2, talent: 2 } : {})
    })
  }
};
