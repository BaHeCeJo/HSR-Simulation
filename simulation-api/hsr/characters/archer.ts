/**
 * @character Archer (EMIYA)
 * @role Main DPS
 * @path The Hunt
 * @element Quantum
 * @core_mechanic "Circuit Connection"  ESkill use enters CC state where turns do not end and
 *   each subsequent Skill deal +100% more DMG (stacks ÁE, ÁE at E6). Exits after 5 active
 *   Skill uses or when SP is exhausted.
 *   "Charges" (max 4) fuel Talent Follow-up ATKs that fire after any teammate action.
 * @skill_priority Ultimate (build charges) ↁESkill (CC chain) ↁEBasic
 * @eidolon_milestones
 *   E1  EAfter 3 Skill uses in one CC turn: recover 2 SP.
 *   E2  EUltimate: ∁E0% enemy Quantum RES + Quantum Weakness for 2 turns.
 *   E3  ESkill +2 (max 15), Basic +1 (max 10).
 *   E4  EUltimate DMG +150%.
 *   E5  EUltimate +2 (max 15), Talent +2 (max 15).
 *   E6  ETurn start: recover 1 SP. CC DMG boost stacks +1 (max 3). Skill DEF ignore +20%.
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy, Action } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

// ── UUID constants ─────────────────────────────────────────────────────────────
const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';

export const ARCHER_ID = 'e1f2a3b4-c5d6-e7f8-a9b0-c1d2e3f4a5b6';

// ── State key helpers ──────────────────────────────────────────────────────────
const CC_KEY       = 'archer_cc';           // 1 = Circuit Connection active
const CC_USES_KEY  = 'archer_cc_uses';      // Skill uses in current CC rotation (1 E)
const CC_DMG_KEY   = 'archer_cc_dmg';      // DMG boost stacks (0  E2 or 3)
const CHARGE_KEY   = 'archer_charges';      // 0 E Charges for Talent FUA
const FUA_SP_KEY   = 'archer_fua_sp';       // Pending SP recoveries from unresolved FUAs
const E1_KEY       = 'archer_e1_done';      // E1 SP refund triggered this CC rotation (0/1)
const ENERGY_KEY   = (id: string) => `archer_energy_${id}`;

// ── Energy helpers ─────────────────────────────────────────────────────────────
const ENERGY_CAP = 220;

const gainEnergy = (s: SimState, member: TeamMember, amount: number) => {
    const next = Math.min((s.stacks[member.characterId] || 0) + amount, ENERGY_CAP);
    s.stacks[member.characterId] = next;
    s.addLog({ type: 'event', message: `Archer gains ${amount} Energy (${next}/${ENERGY_CAP})` });
};

// ── Charge helpers ─────────────────────────────────────────────────────────────
const getCharges   = (s: SimState) => s.stacks[CHARGE_KEY] || 0;
const addCharge    = (s: SimState, n = 1) => {
    s.stacks[CHARGE_KEY] = Math.min(getCharges(s) + n, 4);
};
const consumeCharge = (s: SimState) => {
    s.stacks[CHARGE_KEY] = Math.max(getCharges(s) - 1, 0);
};

// ── CC state helpers ───────────────────────────────────────────────────────────
const inCC      = (s: SimState) => (s.stacks[CC_KEY] || 0) === 1;
const getCCUses = (s: SimState) => s.stacks[CC_USES_KEY] || 0;
const getCCDmg  = (s: SimState) => s.stacks[CC_DMG_KEY]  || 0;

const enterCC = (s: SimState) => {
    s.stacks[CC_KEY]      = 1;
    s.stacks[CC_USES_KEY] = 1;
    s.stacks[CC_DMG_KEY]  = 0;
    s.stacks[E1_KEY]      = 0;
    s.addLog({ type: 'event', message: `[Circuit Connection] Archer enters CC state.` });
};

const exitCC = (s: SimState) => {
    s.stacks[CC_KEY]      = 0;
    s.stacks[CC_USES_KEY] = 0;
    s.stacks[CC_DMG_KEY]  = 0;
    s.stacks[E1_KEY]      = 0;
    s.addLog({ type: 'event', message: `[Circuit Connection] Archer exits CC state.` });
};

/** Max CC DMG boost stacks (base 2, +1 at E6). */
const maxCCDmgStacks = (eidolon: number) => eidolon >= 6 ? 3 : 2;

/**
 * Execute one CC Skill hit.
 * Uses the running CC DMG-boost stacks and applies E6 DEF ignore.
 */
const executeCCSkill = (
    s: SimState, member: TeamMember,
    baseSkillMult: number,
    logLabel: string
) => {
    const target = s.enemies.find(e => e && e.hp > 0);
    if (!target) return;

    const dmgBoostStacks = getCCDmg(s);
    const buffs = { ...member.buffs };
    // CC DMG bonus: +100% per stack
    buffs.dmg_boost += dmgBoostStacks * 100;
    // A6 CRIT DMG buff (already applied if active via buffDurations ↁEonBeforeAction copied into member.buffs)
    // E6: Skill ignores 20% DEF
    if (member.eidolon >= 6) {
        buffs.def_ignore += 20;
    }

    const tempMember = { ...member, buffs };
    const result = calculateHsrDamage({
        character: tempMember, lightcone: member.lightcone,
        enemy: target, ability_multiplier: baseSkillMult, scaling_stat_id: ATK_ID,
    });

    s.totalDamage += result.expected_dmg;
    target.hp = Math.max(0, Math.floor(target.hp - result.expected_dmg));
    s.addLog({
        type: 'event',
        message: `${logLabel} on ${target.name} -> ${result.expected_dmg.toLocaleString()} DMG (CC stack ÁE{dmgBoostStacks})`,
    });
};

// ── A6: SP threshold CRIT DMG buff ────────────────────────────────────────────
/**
 * Apply A6 buff if the current SP is ≥ 4.
 * Stored as a 1-turn buffDuration entry; picked up in onBeforeAction.
 */
const checkAndApplyA6 = (s: SimState, member: TeamMember) => {
    if (s.skillPoints >= 4) {
        s.buffDurations[member.characterId] = s.buffDurations[member.characterId] || {};
        s.buffDurations[member.characterId]['archer_a6'] = { duration: 1, value: 120, stat: 'CRIT DMG' };
        s.addLog({ type: 'event', message: `A6 (SP ≥ 4): Archer gains +120% CRIT DMG for 1 turn.` });
    }
};

// ── Character Kit ──────────────────────────────────────────────────────────────
export const Archer: CharacterKit = {
    id: ARCHER_ID,
    name: 'Archer',
    path: 'The Hunt',
    element: 'Quantum',

    slot_names: {
        basic:    'Kanshou and Bakuya',
        skill:    'Caladbolg II: Fake Spiral Sword',
        ultimate: 'Unlimited Blade Works',
        talent:   "Mind's Eye (True)",
    },

    abilities: {
        basic: {
            ability_id: 'ABILITY_ID_ARCHER_BASIC',
            attribute_index: 0,
            default_multiplier: 1.00, // 100% ATK  ELv.6
            stat_id: ATK_ID,
            distribution: { hits: [1.0] },
            targetType: 'SingleTarget',
            toughness_damage: 10,
        },
        skill: {
            main: {
                ability_id: 'ABILITY_ID_ARCHER_SKILL',
                attribute_index: 0,
                default_multiplier: 3.60, // 360% ATK  ELv.10
                stat_id: ATK_ID,
                targetType: 'SingleTarget',
                toughness_damage: 20,
            },
        },
        ultimate: {
            main: {
                ability_id: 'ABILITY_ID_ARCHER_ULT',
                attribute_index: 0,
                default_multiplier: 10.00, // 1000% ATK  ELv.10
                stat_id: ATK_ID,
                targetType: 'SingleTarget',
                toughness_damage: 30,
            },
        },
        talent: {
            follow_up: {
                ability_id: 'ABILITY_ID_ARCHER_TALENT',
                attribute_index: 0,
                default_multiplier: 2.00, // 200% ATK  ELv.10
                stat_id: ATK_ID,
                toughness_damage: 10,
            },
        },
    },

    hooks: {
        // ────────────────────────────────────────────────────────────────────────
        onBattleStart: (state, member) => {
            // Initialise energy
            state.stacks[member.characterId] = 0;
            // Initialise charges
            state.stacks[CHARGE_KEY] = 0;

            // A4: Gains 1 Charge on battle start
            addCharge(state, 1);
            state.addLog({ type: 'event', message: `A4: Archer starts with 1 Charge (Total: ${getCharges(state)}/4).` });
        },

        // ────────────────────────────────────────────────────────────────────────
        onTurnStart: (state, member) => {
            // E6: Recover 1 SP for allies at turn start
            if (member.eidolon >= 6) {
                state.skillPoints = Math.min(state.skillPoints + 1, 7); // A2 raises cap to 7
                state.addLog({ type: 'event', message: `E6: Archer's turn start recovers 1 SP for allies (SP: ${state.skillPoints}).` });
                checkAndApplyA6(state, member);
            }
        },

        // ────────────────────────────────────────────────────────────────────────
        onBeforeAction: (state, member, action, target) => {
            // A6: Apply CRIT DMG buff if it was granted last SP-gain check
            const a6Buff = state.buffDurations[member.characterId]?.['archer_a6'];
            if (a6Buff && a6Buff.duration > 0) {
                member.buffs.crit_dmg += a6Buff.value || 120;
            }

            // E4: Ultimate DMG +150%
            if (action.type === 'ultimate' && member.eidolon >= 4) {
                member.buffs.dmg_boost += 150;
            }

            // E6: Skill DEF ignore 20% (applied via executeCCSkill for CC hits,
            //     also applied here for the first/only Skill hit in the normal flow)
            if (action.type === 'skill' && member.eidolon >= 6) {
                member.buffs.def_ignore += 20;
            }

            // CC DMG boost for the first Skill hit (CC stacks = 0 on entry, increases in onAfterAction)
            if (action.type === 'skill' && inCC(state)) {
                member.buffs.dmg_boost += getCCDmg(state) * 100;
            }
        },

        // ────────────────────────────────────────────────────────────────────────
        onAfterAction: (state, member, action) => {
            if (action.type === 'basic') {
                // Basic gives 1 SP (handled by simulator). Check A6.
                checkAndApplyA6(state, member);
                gainEnergy(state, member, 20);
            }

            if (action.type === 'skill') {
                if (!inCC(state)) {
                    // First Skill use: enter Circuit Connection (use #1, DMG stack 0)
                    enterCC(state);
                } else {
                    // Subsequent CC Skill uses: increment DMG stack (up to max)
                    const max = maxCCDmgStacks(member.eidolon);
                    state.stacks[CC_DMG_KEY] = Math.min(getCCDmg(state) + 1, max);
                    state.stacks[CC_USES_KEY] = getCCUses(state) + 1;
                }

                const uses = getCCUses(state);
                state.addLog({
                    type: 'event',
                    message: `[CC] Skill use #${uses}/5  EDMG stacks: ${getCCDmg(state)}/${maxCCDmgStacks(member.eidolon)}. SP remaining: ${state.skillPoints}`,
                });

                // E1: After 3 Skill uses in one CC turn ↁErecover 2 SP (once per rotation)
                if (member.eidolon >= 1 && uses >= 3 && !state.stacks[E1_KEY]) {
                    state.stacks[E1_KEY] = 1;
                    const maxSP = 7; // A2 raises cap
                    state.skillPoints = Math.min(state.skillPoints + 2, maxSP);
                    state.addLog({ type: 'event', message: `E1: Recovered 2 SP after 3rd Skill use (SP: ${state.skillPoints}).` });
                    checkAndApplyA6(state, member);
                }

                // Decide whether to continue CC or exit
                const canContinue = uses < 5 && state.skillPoints > 0;
                if (canContinue) {
                    // Continue CC: grant a high-priority extra turn (turn doesn't end)
                    state.grantExtraTurn(member.characterId, '', {
                        isLowPriority: false,
                        reason: `Circuit Connection [${uses + 1}/5]`,
                    });
                } else {
                    // Exit CC
                    exitCC(state);
                }

                gainEnergy(state, member, 30);
            }
        },

        // ────────────────────────────────────────────────────────────────────────
        onUlt: (state, member) => {
            const target = state.enemies.find(e => e && e.hp > 0);
            if (!target) return;

            const ultMult = 10.00; // 1000% ATK  ELv.10
            const buffs = { ...member.buffs };

            // E4: Ult DMG +150% (also applied in onBeforeAction; buffsMember already has it)
            // No additional application needed here since onBeforeAction ran first.

            // E2: ∁E0% Quantum RES + Quantum Weakness for 2 turns
            if (member.eidolon >= 2) {
                const prevQRes = target.elemental_res['Quantum'] ?? target.resistance;
                target.elemental_res['Quantum'] = prevQRes - 0.20;
                if (!target.weaknesses.includes('Quantum')) {
                    target.weaknesses.push('Quantum');
                }
                target.activeDebuffs['archer_e2_qres'] = {
                    duration: 2, value: 20, stat: 'Quantum RES',
                };
                target.debuffCount = Object.keys(target.activeDebuffs).length;
                state.addLog({ type: 'event', message: `E2: ${target.name} Quantum RES ∁E0% and Quantum Weakness for 2 turns.` });
            }

            const tempMember = { ...member, buffs };
            const result = calculateHsrDamage({
                character: tempMember, lightcone: member.lightcone,
                enemy: target, ability_multiplier: ultMult, scaling_stat_id: ATK_ID,
            });

            state.totalDamage += result.expected_dmg;
            target.hp = Math.max(0, Math.floor(target.hp - result.expected_dmg));
            state.addLog({
                type: 'event',
                message: `Ultimate on ${target.name} -> ${result.expected_dmg.toLocaleString()} DMG`,
            });

            // Toughness (30)
            if ((state as any).applyToughnessDamage) {
                (state as any).applyToughnessDamage(target, 30, false);
            }

            // Gains 2 Charges (max 4)
            addCharge(state, 2);
            state.addLog({ type: 'event', message: `Ultimate grants 2 Charges (Total: ${getCharges(state)}/4).` });

            // Energy: consumed 220, gain 5
            state.stacks[member.characterId] = 5;

            // Also exit CC if active when ult is used
            if (inCC(state)) exitCC(state);
        },

        // ────────────────────────────────────────────────────────────────────────
        onExtraTurn: (state, member) => {
            // All of Archer's extra turns are Circuit Connection continuations.
            if (!inCC(state)) {
                // Fallback for non-CC extra turns: basic attack
                const target = state.enemies.find(e => e && e.hp > 0);
                if (!target) return;
                const buffs = { ...member.buffs };
                const a6Buff = state.buffDurations[member.characterId]?.['archer_a6'];
                if (a6Buff?.duration > 0) buffs.crit_dmg += a6Buff.value || 120;
                const tempMember = { ...member, buffs };
                const result = calculateHsrDamage({
                    character: tempMember, lightcone: member.lightcone,
                    enemy: target, ability_multiplier: 1.00, scaling_stat_id: ATK_ID,
                });
                state.totalDamage += result.expected_dmg;
                target.hp = Math.max(0, Math.floor(target.hp - result.expected_dmg));
                state.addLog({ type: 'event', message: `[Extra Turn] Basic on ${target.name} -> ${result.expected_dmg.toLocaleString()} DMG` });
                gainEnergy(state, member, 20);
                return;
            }

            // ── Circuit Connection extra Skill ──────────────────────────────────
            // Consume 1 SP for this CC Skill use
            if (state.skillPoints <= 0) {
                exitCC(state);
                return;
            }
            state.skillPoints--;

            const baseSkillMult = 3.60; // 360%  ELv.10 (resolved from kit default since no DB ability for CC hits)
            const ccUsesBefore = getCCUses(state);
            const label = `[CC Skill ${ccUsesBefore + 1}/5]`;

            // Increment DMG stack BEFORE dealing damage (each CC re-use adds +100%)
            const max = maxCCDmgStacks(member.eidolon);
            state.stacks[CC_DMG_KEY] = Math.min(getCCDmg(state) + 1, max);
            state.stacks[CC_USES_KEY] = ccUsesBefore + 1;

            executeCCSkill(state, member, baseSkillMult, label);

            const uses = getCCUses(state);
            state.addLog({
                type: 'event',
                message: `[CC] Skill use #${uses}/5  EDMG stack: ${getCCDmg(state)}/${max}. SP remaining: ${state.skillPoints}`,
            });

            // E1: After 3rd Skill use in the rotation ↁErecover 2 SP (once per CC)
            if (member.eidolon >= 1 && uses >= 3 && !state.stacks[E1_KEY]) {
                state.stacks[E1_KEY] = 1;
                state.skillPoints = Math.min(state.skillPoints + 2, 7);
                state.addLog({ type: 'event', message: `E1: Recovered 2 SP after 3rd CC Skill use (SP: ${state.skillPoints}).` });
                checkAndApplyA6(state, member);
            }

            // Toughness for this CC Skill hit
            const target = state.enemies.find(e => e && e.hp > 0);
            if (target && (state as any).applyToughnessDamage) {
                (state as any).applyToughnessDamage(target, 20, false);
            }

            // Energy per CC Skill use: 30
            gainEnergy(state, member, 30);

            // Continue or exit CC
            const canContinue = uses < 5 && state.skillPoints > 0;
            if (canContinue) {
                state.grantExtraTurn(member.characterId, '', {
                    isLowPriority: false,
                    reason: `Circuit Connection [${uses + 1}/5]`,
                });
            } else {
                exitCC(state);
            }
        },

        // ────────────────────────────────────────────────────────────────────────
        onCheckFollowUp: (state, member, trigger) => {
            // ── Talent: Follow-up when a TEAMMATE attacks ───────────────────────
            if (trigger === 'after_action') {
                // Only trigger when it's NOT Archer's own action.
                // currentActionId format: "${characterId}-${av}-${actionType}"
                // ARCHER_ID is 36 chars; the ID occupies exactly those first 36 chars.
                const archerActing = (state.currentActionId?.slice(0, ARCHER_ID.length) ?? '') === ARCHER_ID;
                if (!archerActing && getCharges(state) > 0) {
                    consumeCharge(state);
                    const target = state.enemies.find(e => e && e.hp > 0);
                    if (target) {
                        // Mark that this FUA will recover 1 SP on resolution
                        state.stacks[FUA_SP_KEY] = (state.stacks[FUA_SP_KEY] || 0) + 1;
                        state.queueFollowUp({
                            actorId: ARCHER_ID,
                            actorInstanceId: '',
                            action: {
                                type: 'follow_up',
                                multiplier: 2.00, // 200% ATK  ETalent Lv.10
                                stat_id: ATK_ID,
                                toughness_damage: 10,
                                inflictsDebuff: false,
                            },
                            targetInstanceId: target.instanceId,
                        });
                        state.addLog({
                            type: 'event',
                            message: `[Talent] Archer queues FUA (${getCharges(state)} Charges remaining). Recovers 1 SP on hit.`,
                        });
                    } else {
                        // Target died before FUA; redirect to random enemy (handled by queueFollowUp default)
                        state.stacks[FUA_SP_KEY] = (state.stacks[FUA_SP_KEY] || 0) + 1;
                        state.queueFollowUp({
                            actorId: ARCHER_ID,
                            actorInstanceId: '',
                            action: {
                                type: 'follow_up',
                                multiplier: 2.00,
                                stat_id: ATK_ID,
                                toughness_damage: 10,
                                inflictsDebuff: false,
                            },
                        });
                    }
                }
            }

            // ── SP recovery from resolved FUAs ──────────────────────────────────
            if (trigger === 'after_follow_up') {
                const pending = state.stacks[FUA_SP_KEY] || 0;
                if (pending > 0) {
                    state.stacks[FUA_SP_KEY] = pending - 1;
                    const maxSP = 7; // A2 raises cap to 7
                    state.skillPoints = Math.min(state.skillPoints + 1, maxSP);
                    state.addLog({ type: 'event', message: `[Talent FUA] Recovers 1 SP (SP: ${state.skillPoints}).` });
                    checkAndApplyA6(state, member);
                    // Talent also generates 5 energy
                    gainEnergy(state, member, 5);
                }
            }
        },

        // ────────────────────────────────────────────────────────────────────────
        onEnemyDefeated: (state, _member, _enemy) => {
            // "Exits the Circuit Connection state after all enemy targets have been defeated
            //  in each wave."  Ehandled naturally: no alive enemies ↁECC extra turns resolve
            //  to nothing and exitCC is called. But explicitly clear CC here for safety.
            const allDead = state.enemies.every(e => !e || e.hp <= 0);
            if (allDead && inCC(state)) {
                exitCC(state);
            }
        },
    },

    special_modifiers: {
        energy_type: 'NONE',    // Managed manually; state.stacks[characterId] used for ult check
        energy_cost: 220,

        // A2: Max SP +2 (from 5 to 7). The simulator's SP cap is 5 by default; Archer's
        //     presence implicitly allows up to 7 SP via the +1 gains from FUA and E6 turn-start.
        //     No code change needed in the simulator's min/max since the extra SP is just retained.

        eidolon_level_boosts: (eidolon: number) => ({
            ...(eidolon >= 3 ? { skill: 2, basic: 1 } : {}),
            ...(eidolon >= 5 ? { ultimate: 2, talent: 2 } : {}),
        }),

        // Minor traces  EQuantum DMG as dmg_boost (Archer deals only Quantum DMG)
        stat_boosts: () => ({
            dmg_boost: 22.4,  // +22.4% Quantum DMG Boost
            atk_percent: 18,  // +18% ATK
            crit_rate: 6.7,   // +6.7% CRIT Rate
        }),
    },
};
