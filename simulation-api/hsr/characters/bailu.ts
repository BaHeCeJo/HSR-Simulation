/**
 * @character Bailu
 * @role Sustain / Abundance (Lightning)
 * @core_mechanics
 *   All heals scale off Bailu's Max HP (use member.max_hp  Ecaller must include traces).
 *   Invigoration: applied by ult and technique.  Enables talent procs, A6 DMG reduction.
 *   Talent: killing blow prevention + heal (1 per battle, 2 at E6).
 *
 * Implemented:
 *   Basic    E100% ATK Lightning, single target.
 *   Skill    EHeals lowest-HP ally (11.7% HP+312), then 2 random heals at ÁE.85 decay.
 *             E4: each heal grants +10% DMG to recipient (stack to 3, persistent in sim).
 *   Ult      EHeals all allies (13.5% HP+360); applies Invigoration (2 turns, or extends
 *             existing by +1). Resets talent trigger counter. E2: +15% heals for 2 turns.
 *   Talent   EonEnemyAction: heal lowest-HP Invigorated ally (5.4% HP+144, 3 times per
 *             Invigoration cycle  Ebase 2 + A4).
 *             KO prevention: absorbed in applyDamageToAlly wrap (18% HP+480 heal).
 *   A4       E+1 talent trigger (always active as major trace ↁEbase becomes 3).
 *   A6       EInvigorated allies take 10% less DMG (handled in applyDamageToAlly wrap).
 *   E2       EAfter ult: Outgoing Healing +15% for 2 turns.
 *   E4       EEach Skill heal: +10% DMG to recipient (max 3 stacks = +30%).
 *   E6       EKO survival +1 extra time per battle.
 *
 * Invigoration is applied at battle start (simulating technique) and on each ult.
 * Invigoration duration is tracked in state.buffDurations per ally and auto-decrements.
 * Energy: Basic +20 (auto), Skill +30 (auto), Ult +5 (onUlt).
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';

const BAILU_ID = 'd3e4f5a6-b7c8-4901-d2e3-f4a5b6c7d8e9';

// State keys
const TALENT_TRIGGERS_KEY = 'bailu_talent_triggers';
const REVIVES_KEY          = 'bailu_revives';
const E2_TURNS_KEY         = 'bailu_e2_turns';

/** Base talent trigger count: 2 + A4 (+1 from major trace, always active) = 3. */
const TALENT_TRIGGER_MAX = 3;

// ── Helpers ───────────────────────────────────────────────────────────────────

function getBailu(state: SimState): TeamMember | undefined {
  return state.team.find(m => m.characterId === BAILU_ID);
}

function isInvigorated(state: SimState, member: TeamMember): boolean {
  return !!(state.buffDurations[member.characterId]?.['bailu_invigoration']);
}

/**
 * Heal `target` for (pct ÁEbailu.max_hp + flat), applying E2 boost if active.
 * Returns the actual heal amount applied.
 */
function doHeal(state: SimState, bailu: TeamMember, target: TeamMember, pct: number, flat: number): number {
  if (target.isDowned) return 0;
  let amount = Math.floor(pct * bailu.max_hp + flat);
  if ((state.stacks[E2_TURNS_KEY] || 0) > 0) {
    amount = Math.floor(amount * 1.15);
  }
  const actual = Math.min(amount, Math.max(0, target.max_hp - target.hp));
  target.hp = Math.min(target.max_hp, target.hp + amount);
  return actual;
}

/**
 * Apply or extend Invigoration to a single ally.
 *  EIf already Invigorated: extend duration by +1.
 *  EIf not Invigorated: set duration = 2.
 */
function applyInvigoration(state: SimState, ally: TeamMember): void {
  if (!state.buffDurations[ally.characterId]) state.buffDurations[ally.characterId] = {};
  const existing = state.buffDurations[ally.characterId]['bailu_invigoration'];
  if (existing) {
    existing.duration++;
    state.addLog({ type: 'event', message: `Bailu Ult: Invigoration extended for ${ally.name} (${existing.duration} turns left).` });
  } else {
    state.buffDurations[ally.characterId]['bailu_invigoration'] = { duration: 2, stat: 'Invigoration' };
    state.addLog({ type: 'event', message: `Bailu Ult: Invigoration applied to ${ally.name} (2 turns).` });
  }
}

// ── Kit ───────────────────────────────────────────────────────────────────────

export const Bailu: CharacterKit = {
  id: BAILU_ID,
  name: "Bailu",
  path: "Abundance",
  element: "Lightning",
  slot_names: {
    basic:    "Diagnostic Kick",
    skill:    "Singing Among Clouds",
    ultimate: "Felicitous Thunderleap",
    talent:   "Gourdful of Elixir",
  },
  abilities: {
    basic: {
      default_multiplier: 1.0, // Lv.6: 100% ATK, Lightning damage
      stat_id: ATK_ID,
      targetType: 'SingleTarget',
      toughness_damage: 10
    },
    skill: {
      // No damage  Ehealing handled entirely in onAfterAction.
      // Nested structure avoids falsy default_multiplier: 0 misparse.
      main: {
        default_multiplier: 0,
        stat_id: ATK_ID,
        targetType: 'Restore'
      }
    },
    ultimate: {
      // Handled in onUlt; definition is never used by the simulator when onUlt is defined.
      default_multiplier: 0,
      stat_id: ATK_ID,
      targetType: 'Restore'
    },
    talent: {
      default_multiplier: 0,
      stat_id: ATK_ID
    }
  },

  hooks: {
    // ── Battle Start ──────────────────────────────────────────────────────────
    onBattleStart: (state, member) => {
      // Initialize counters
      state.stacks[TALENT_TRIGGERS_KEY] = TALENT_TRIGGER_MAX;
      state.stacks[REVIVES_KEY] = member.eidolon >= 6 ? 2 : 1;
      state.stacks[E2_TURNS_KEY] = 0;

      // Technique: apply battle-start Invigoration to all allies (2 turns)
      state.team.forEach(m => {
        if (!m.isDowned) {
          if (!state.buffDurations[m.characterId]) state.buffDurations[m.characterId] = {};
          state.buffDurations[m.characterId]['bailu_invigoration'] = { duration: 2, stat: 'Invigoration' };
        }
      });
      state.addLog({ type: 'event', message: `Bailu Technique: All allies start with Invigoration (2 turns).` });

      // A6: Wrap applyDamageToAlly to handle DMG reduction and KO prevention
      const originalApply = state.applyDamageToAlly!;
      state.applyDamageToAlly = (target: TeamMember, damage: number, toughnessDamage?: number) => {
        if (target.isDowned) { originalApply(target, damage, toughnessDamage); return; }

        // A6: Invigorated allies take 10% less DMG
        let actualDmg = damage;
        if (isInvigorated(state, target)) {
          actualDmg = Math.floor(damage * 0.9);
        }

        // Talent: Killing blow prevention
        if (target.hp - actualDmg <= 0 && (state.stacks[REVIVES_KEY] || 0) > 0) {
          const bailu = getBailu(state);
          if (bailu) {
            state.stacks[REVIVES_KEY]--;
            target.hp = 1;
            const healAmt = doHeal(state, bailu, target, 0.18, 480);
            state.addLog({
              type: 'event',
              message: `Bailu Talent: KO prevented on ${target.name}! Healed +${healAmt.toLocaleString()} HP (HP: ${target.hp.toLocaleString()}/${target.max_hp.toLocaleString()}).`
            });
            // Still apply toughness even though HP was saved
            if (toughnessDamage && toughnessDamage > 0) originalApply(target, 0, toughnessDamage);
            return;
          }
        }

        originalApply(target, actualDmg, toughnessDamage);
      };
    },

    // ── Turn Start ────────────────────────────────────────────────────────────
    onTurnStart: (state, member) => {
      // E2: Decrement Outgoing Healing boost
      if ((state.stacks[E2_TURNS_KEY] || 0) > 0) {
        state.stacks[E2_TURNS_KEY]--;
        if (state.stacks[E2_TURNS_KEY] <= 0) {
          state.addLog({ type: 'event', message: `Bailu E2: Outgoing Healing +15% expired.` });
        }
      }
    },

    // ── After Action ──────────────────────────────────────────────────────────
    onAfterAction: (state, member, action) => {
      // ── SKILL: 3-hit cascade heal ───────────────────────────────────────────
      if (action.type === 'skill') {
        const bailu = getBailu(state);
        if (!bailu) return;

        const aliveTeam = state.team.filter(m => !m.isDowned);
        if (aliveTeam.length === 0) return;

        // Base heal per hit (before 0.85 decay)
        const baseHeal = 0.117 * bailu.max_hp + 312;

        // 1st heal: primary target = lowest HP alive ally
        const primaryTarget = aliveTeam.reduce((min, m) => m.hp < min.hp ? m : min);
        const h1 = doHeal(state, bailu, primaryTarget, 0.117, 312);
        state.addLog({ type: 'event', message: `Skill heal 1: ${primaryTarget.name} +${h1.toLocaleString()} HP (HP: ${primaryTarget.hp.toLocaleString()}/${primaryTarget.max_hp.toLocaleString()})` });
        applyE4(state, member, primaryTarget);

        // 2nd heal: random ally, reduced by 15%
        const secondHealAmt = baseHeal * 0.85;
        const pct2 = secondHealAmt / (bailu.max_hp || 1);
        const t2 = aliveTeam[Math.floor(Math.random() * aliveTeam.length)];
        const h2 = doHeal(state, bailu, t2, pct2, 0);
        state.addLog({ type: 'event', message: `Skill heal 2: ${t2.name} +${h2.toLocaleString()} HP (ÁE.85, HP: ${t2.hp.toLocaleString()}/${t2.max_hp.toLocaleString()})` });
        applyE4(state, member, t2);

        // 3rd heal: random ally, further reduced by 15% (ÁE.85²)
        const thirdHealAmt = baseHeal * 0.7225;
        const pct3 = thirdHealAmt / (bailu.max_hp || 1);
        const t3 = aliveTeam[Math.floor(Math.random() * aliveTeam.length)];
        const h3 = doHeal(state, bailu, t3, pct3, 0);
        state.addLog({ type: 'event', message: `Skill heal 3: ${t3.name} +${h3.toLocaleString()} HP (ÁE.72, HP: ${t3.hp.toLocaleString()}/${t3.max_hp.toLocaleString()})` });
        applyE4(state, member, t3);

        state.addLog({
          type: 'event',
          message: `Singing Among Clouds: ${(h1 + h2 + h3).toLocaleString()} total healing.`
        });
      }
    },

    // ── Ultimate ──────────────────────────────────────────────────────────────
    onUlt: (state, member) => {
      const bailu = getBailu(state);
      if (!bailu) return;

      // Energy reset: ult costs 100, grants 5
      state.stacks[BAILU_ID] = 5;

      // E2: +15% Outgoing Healing for 2 turns after ult
      if (member.eidolon >= 2) {
        state.stacks[E2_TURNS_KEY] = 2;
        state.addLog({ type: 'event', message: `Bailu E2: Outgoing Healing +15% for 2 turns.` });
      }

      // Heal all alive allies: 13.5% MaxHP + 360
      let totalHeal = 0;
      state.team.forEach(m => {
        if (!m.isDowned) {
          const h = doHeal(state, bailu, m, 0.135, 360);
          totalHeal += h;
        }
      });
      state.addLog({ type: 'event', message: `Felicitous Thunderleap: ${totalHeal.toLocaleString()} total healing to all allies.` });

      // Apply or extend Invigoration to all alive allies
      state.team.forEach(m => {
        if (!m.isDowned) applyInvigoration(state, m);
      });

      // Reset talent trigger counter on Invigoration refresh
      state.stacks[TALENT_TRIGGERS_KEY] = TALENT_TRIGGER_MAX;
      state.addLog({ type: 'event', message: `Bailu: Talent triggers reset to ${TALENT_TRIGGER_MAX}.` });
    },

    // ── Enemy Action: Talent Invigoration Proc ────────────────────────────────
    onEnemyAction: (state, member, enemy) => {
      const bailu = getBailu(state);
      if (!bailu) return;

      const triggersLeft = state.stacks[TALENT_TRIGGERS_KEY] || 0;
      if (triggersLeft <= 0) return;

      // Find all Invigorated alive allies
      const invigoratedAllies = state.team.filter(m => !m.isDowned && isInvigorated(state, m));
      if (invigoratedAllies.length === 0) return;

      // Heal the Invigorated ally with the lowest HP (most in need)
      const target = invigoratedAllies.reduce((min, m) => m.hp < min.hp ? m : min);

      const healAmt = doHeal(state, bailu, target, 0.054, 144);
      state.stacks[TALENT_TRIGGERS_KEY]--;

      state.addLog({
        type: 'event',
        message: `Bailu Talent: Invigorated ${target.name} healed +${healAmt.toLocaleString()} HP (${state.stacks[TALENT_TRIGGERS_KEY]} trigger(s) left).`
      });
    }
  },

  special_modifiers: {
    energy_type: "ENERGY",
    energy_cost: 100,
    stat_boosts: (stats: any) => ({
      // Minor traces: HP +28% (not tracked  Ehandled via max_hp in caller),
      // DEF +22.5%, Effect RES +10% (not in combat buffs struct)
      // No ATK%, CRIT Rate, CRIT DMG, or DMG Boost from traces
    }),
    eidolon_level_boosts: (eidolon: number) => ({
      ...(eidolon >= 3 ? { skill: 2, talent: 2 } : {}),
      ...(eidolon >= 5 ? { ultimate: 2, basic: 1 } : {})
    })
  }
};

// ── E4 helper (defined after the kit to keep closure clean) ──────────────────

/**
 * E4: Each skill heal grants +10% DMG to the recipient for 2 turns (up to 3 stacks).
 * Stack count is tracked per ally; expiry is not modeled (persistent in simulation).
 */
function applyE4(state: SimState, bailu: TeamMember, target: TeamMember): void {
  if (bailu.eidolon < 4) return;
  const stackKey = 'bailu_e4_' + target.characterId;
  const stacks = state.stacks[stackKey] || 0;
  if (stacks >= 3) return;
  state.stacks[stackKey] = stacks + 1;
  target.buffs.dmg_boost += 10;
  state.addLog({
    type: 'event',
    message: `Bailu E4: ${target.name} DMG +10% from skill heal (stack ${stacks + 1}/3).`
  });
}
