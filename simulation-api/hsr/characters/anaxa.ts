/**
 * @character Anaxa
 * @role Main DPS / Debuffer
 * @path Erudition
 * @element Wind
 * @core_mechanic Per-hit Weakness implantation (Talent). "Qualitative Disclosure" activates on
 *   enemies with ≥5 different Weakness Types  EAnaxa deals +30% DMG to them and fires 1 free
 *   extra Skill after using Basic or Skill on them (cannot chain).
 * @skill_priority Ultimate ↁESkill ↁEBasic
 * @eidolon_milestones
 *   E1  EAfter first Skill: recover 1 SP. Skill hits: enemy DEF ∁E6% for 2 turns.
 *   E2  EOn battle start: trigger 1 Weakness implant + ∁E0% All-Type RES on every enemy.
 *   E3  EUltimate +2 (max 15), Basic +1 (max 10).
 *   E4  ESkill use: ATK +30% for 2 turns, stacks up to 2ÁE
 *   E5  ESkill +2 (max 15), Talent +2 (max 15).
 *   E6  EAll Anaxa DMG ÁE.3. Both A4 Trace effects active unconditionally.
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

// ── UUID constants ─────────────────────────────────────────────────────────────
const ATK_ID      = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const CRIT_DMG_ID = 'a93e523a-7852-4580-b2ef-03467e214bcd';

export const ANAXA_ID = '7e8f9a0b-1c2d-3e4f-5a6b-7c8d9e0f1a2b';

// All weakness types in the order they are implanted
const ALL_ELEMENTS: string[] = [
    "Physical", "Fire", "Ice", "Lightning", "Wind", "Quantum", "Imaginary"
];

// ── Weakness / QD helpers ──────────────────────────────────────────────────────

/** Number of unique Weakness Types Anaxa has implanted on this enemy (0 E). */
const getWkCount = (s: SimState, iid: string): number =>
    s.stacks[`anaxa_wk_${iid}`] || 0;

/** Increment Anaxa's weakness-type count on an enemy by n, capped at 7. */
const addWkCount = (s: SimState, iid: string, n = 1) => {
    s.stacks[`anaxa_wk_${iid}`] = Math.min(getWkCount(s, iid) + n, 7);
};

/** True when the enemy has ≥5 Weakness Types ↁEeligible for Qualitative Disclosure. */
const hasQD = (s: SimState, iid: string): boolean =>
    getWkCount(s, iid) >= 5;

/**
 * Implant n Weakness types on an enemy, append them to enemy.weaknesses[],
 * and trigger Qualitative Disclosure the first time ≥5 types are reached.
 */
const implantWeakness = (
    s: SimState, member: TeamMember, enemy: SimEnemy, n = 1
) => {
    const prev = getWkCount(s, enemy.instanceId);
    addWkCount(s, enemy.instanceId, n);
    const next = getWkCount(s, enemy.instanceId);

    // Append concrete weakness type strings (used by toughness reduction checks)
    for (let i = prev; i < next; i++) {
        const type = ALL_ELEMENTS[i];
        if (type && !enemy.weaknesses.includes(type)) {
            enemy.weaknesses.push(type);
        }
    }

    // First time ≥5 ↁEQualitative Disclosure
    if (next >= 5 && !s.stacks[`anaxa_qd_active_${enemy.instanceId}`]) {
        s.stacks[`anaxa_qd_active_${enemy.instanceId}`] = 1;
        enemy.activeDebuffs["Qualitative Disclosure"] = {
            duration: 999, // Managed manually; persists until enemy dies
            stat: "Qualitative Disclosure",
        };
        s.addLog({
            type: 'event',
            message: `[Qualitative Disclosure] ${enemy.name} now has ${next} Weakness Types!`,
        });
    }
};

// ── Energy helpers ─────────────────────────────────────────────────────────────
// Energy is tracked in state.stacks[characterId] (same key the simulator uses for ult readiness).

const gainEnergy = (s: SimState, member: TeamMember, amount: number) => {
    const cap = 140;
    const next = Math.min((s.stacks[member.characterId] || 0) + amount, cap);
    s.stacks[member.characterId] = next;
    s.addLog({ type: 'event', message: `Anaxa gains ${amount} Energy (${next}/${cap})` });
};

// ── E4 ATK stack helper ────────────────────────────────────────────────────────

/** Returns how many active E4 ATK-buff stacks Anaxa currently has (0 E). */
const getE4Stacks = (s: SimState, member: TeamMember): number => {
    if (!s.buffDurations[member.characterId]) return 0;
    return Object.keys(s.buffDurations[member.characterId])
        .filter(k => k.startsWith('anaxa_e4_stack_') && s.buffDurations[member.characterId][k].duration > 0)
        .length;
};

// ── Per-hit buff builder ───────────────────────────────────────────────────────
/**
 * Build a temporary buffs snapshot for a single hit on a specific target.
 * Starts from member.buffs (which already contains permanent A4/E6 bonuses from onBattleStart).
 * Adds:
 *   - QD: +30% dmg_boost
 *   - A6: +4% def_ignore per Weakness Type on target (max 7 types = 28%)
 *   - E1 DEF shred: +16% def_reduction if anaxa_e1_def debuff is on the target
 *   - E4 ATK stacks: +30% atk_percent per active stack
 *   - Per-enemy Skill bonus: extraDmgBoost (Skill passive: +20% per attackable enemy)
 */
const buildHitBuffs = (
    s: SimState, member: TeamMember, target: SimEnemy,
    extraDmgBoost = 0
) => {
    const buffs = { ...member.buffs };

    // QD +30% DMG
    if (hasQD(s, target.instanceId)) {
        buffs.dmg_boost += 30;
    }

    // A6: +4% DEF ignore per unique Weakness Type (max 7)
    buffs.def_ignore += Math.min(getWkCount(s, target.instanceId), 7) * 4;

    // E1 DEF shred: Skill hits leave a -16% DEF debuff; Anaxa benefits from it on subsequent hits
    if (target.activeDebuffs["anaxa_e1_def"]) {
        buffs.def_reduction += target.activeDebuffs["anaxa_e1_def"].value || 16;
    }

    // E4 ATK buff stacks
    buffs.atk_percent += getE4Stacks(s, member) * 30;

    // Skill per-enemy DMG bonus
    buffs.dmg_boost += extraDmgBoost;

    return buffs;
};

// ── Skill execution (shared for normal Skill and QD extra Skill) ───────────────
/**
 * Execute all 5 hits of Anaxa's Skill:
 *   Hit [0] = main target (first alive enemy).
 *   Hits [1 E] = bounce; prioritise enemies not yet hit this cast.
 *
 * @param isQDExtra  True when called for the QD-triggered extra Skill  Eprevents chaining.
 * @param alreadyHitMain  True when Hit[0] was already resolved by the normal action loop
 *                        (onAfterAction path). Skips recalculating Hit[0] to avoid double-count.
 */
const executeSkill = (
    s: SimState, member: TeamMember, skillMult: number,
    isQDExtra: boolean, alreadyHitMain = false
) => {
    const alive = s.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
    if (alive.length === 0) return;

    const mainTarget = alive[0];
    // Skill passive: +20% DMG per attackable enemy on the field
    const perEnemyBonus = alive.length * 20;

    // Build the 5-hit list
    const hitList: SimEnemy[] = [mainTarget];
    const hitSet = new Set<string>([mainTarget.instanceId]);
    for (let i = 0; i < 4; i++) {
        const live = s.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
        const unhit = live.filter(e => !hitSet.has(e.instanceId));
        const pool = unhit.length > 0 ? unhit : live;
        const pick = pool[Math.floor(Math.random() * pool.length)];
        hitList.push(pick);
        hitSet.add(pick.instanceId);
    }

    // Resolve hits
    hitList.forEach((target, idx) => {
        // Hit 0 was already calculated by the normal action loop  Eonly implant weakness
        if (idx === 0 && alreadyHitMain) {
            implantWeakness(s, member, target, 1);
            if (member.eidolon >= 1) {
                target.activeDebuffs["anaxa_e1_def"] = { duration: 2, value: 16, stat: "DEF reduction" };
                target.debuffCount = Object.keys(target.activeDebuffs).length;
            }
            return;
        }

        const buffs = buildHitBuffs(s, member, target, perEnemyBonus);
        const tempMember = { ...member, buffs };
        const result = calculateHsrDamage({
            character: tempMember, lightcone: member.lightcone,
            enemy: target, ability_multiplier: skillMult, scaling_stat_id: ATK_ID,
        });

        s.totalDamage += result.expected_dmg;
        target.hp = Math.max(0, Math.floor(target.hp - result.expected_dmg));

        const label = isQDExtra ? `QD-Skill` : `Skill`;
        s.addLog({
            type: 'event',
            message: `Hit ${label} [${idx + 1}/5] on ${target.name} -> ${result.expected_dmg.toLocaleString()} DMG`,
        });

        // Talent: 1 Weakness implant per hit
        implantWeakness(s, member, target, 1);

        // E1: DEF shred on every Skill hit
        if (member.eidolon >= 1) {
            target.activeDebuffs["anaxa_e1_def"] = { duration: 2, value: 16, stat: "DEF reduction" };
            target.debuffCount = Object.keys(target.activeDebuffs).length;
        }
    });

    // QD extra Skill trigger: only from a non-QD Skill and only once per action
    if (!isQDExtra && hasQD(s, mainTarget.instanceId)) {
        const triggerKey = `anaxa_qd_trigger_${s.currentActionId}`;
        if (!s.stacks[triggerKey]) {
            s.stacks[triggerKey] = 1;
            s.stacks['anaxa_qd_pending_extra'] = 1;
            s.addLog({ type: 'event', message: `[Qualitative Disclosure] grants 1 extra Skill on ${mainTarget.name}!` });
            s.grantExtraTurn(member.characterId, '', { reason: 'Qualitative Disclosure' });
        }
    }
};

// ── Character Kit ──────────────────────────────────────────────────────────────
export const Anaxa: CharacterKit = {
    id: ANAXA_ID,
    name: 'Anaxa',
    path: 'Erudition',
    element: 'Wind',

    slot_names: {
        basic:    'Pain, Brews Truth',
        skill:    'Fractal, Exiles Fallacy',
        ultimate: 'Sprouting Life Sculpts Earth',
        talent:   'Tetrad Wisdom Reigns Thrice',
    },

    abilities: {
        basic: {
            ability_id: 'ABILITY_ID_ANAXA_BASIC',
            attribute_index: 0,
            default_multiplier: 1.00, // 100% ATK  ELv.6
            stat_id: ATK_ID,
            distribution: { hits: [1.0] },
            targetType: 'SingleTarget',
            toughness_damage: 10,
        },
        skill: {
            // Main hit only  E4 bounce hits are resolved in onAfterAction via executeSkill.
            // The per-enemy Skill DMG bonus (+20% per enemy) is applied via onBeforeAction.
            main: {
                ability_id: 'ABILITY_ID_ANAXA_SKILL',
                attribute_index: 0,
                default_multiplier: 0.70, // 70% ATK per hit  ELv.10
                stat_id: ATK_ID,
                targetType: 'SingleTarget',
                toughness_damage: 10, // One toughness roll per Skill (first hit; bounces omitted)
            },
        },
        ultimate: {
            main: {
                ability_id: 'ABILITY_ID_ANAXA_ULT',
                attribute_index: 0,
                default_multiplier: 1.60, // 160% ATK  ELv.10
                stat_id: ATK_ID,
                targetType: 'AoE',
                toughness_damage: 20,
            },
        },
        talent: {
            // Not a damage ability  Elogic is entirely in hooks (implantWeakness calls).
            weakness_implant: {
                ability_id: 'ABILITY_ID_ANAXA_TALENT',
                attribute_index: 0,
                default_multiplier: 0,
                stat_id: ATK_ID,
            },
        },
    },

    hooks: {
        // ────────────────────────────────────────────────────────────────────────
        onBattleStart: (state, member) => {
            // Initialise energy (tracked in state.stacks[characterId] for ult-ready check)
            state.stacks[member.characterId] = 0;

            // ── Count Erudition characters in the team ──────────────────────────
            // TeamMember.path may be set by the front end; ANAXA_ID is always Erudition.
            const eruditionCount = state.team.filter(
                m => (m as any).path === 'Erudition' || m.characterId === ANAXA_ID
            ).length;
            state.stacks['anaxa_erudition_count'] = eruditionCount;

            // ── A4: Imperative Hiatus (permanent for the battle) ────────────────
            // E6 forces both effects regardless of team composition.
            const twoEffect = member.eidolon >= 6 || eruditionCount >= 2;
            const oneEffect = member.eidolon >= 6 || eruditionCount >= 1;

            if (oneEffect) {
                // +140% CRIT DMG for Anaxa (permanently added to member.buffs)
                member.buffs.crit_dmg += 140;
                state.addLog({ type: 'event', message: `A4 (Erudition ≥1): Anaxa gains +140% CRIT DMG.` });
            }
            if (twoEffect) {
                // +50% DMG for all allies  Esimulated as +50% enemy vulnerability
                // (mathematically equivalent when all ally damage targets the same enemy pool)
                state.enemies.forEach(e => { if (e) e.vulnerability += 50; });
                state.addLog({ type: 'event', message: `A4 (Erudition ≥2): All allies +50% DMG (applied as enemy vulnerability).` });
            }

            // ── E6: All Anaxa DMG ÁE.3  Eapproximated as +30% additive DMG boost ──
            // Stored permanently; picked up via buildHitBuffs (which copies member.buffs).
            // Note: true multiplicative stacking is slightly higher than +30% additive
            // when combined with other boosts, but the difference is small (<5%).
            if (member.eidolon >= 6) {
                member.buffs.dmg_boost += 30;
                state.addLog({ type: 'event', message: `E6: Anaxa's DMG ÁE.3 (approximated as permanent +30% DMG Boost).` });
            }

            // ── E2: Weakness implant + RES reduction on every initial enemy ───────
            if (member.eidolon >= 2) {
                state.enemies.forEach(e => {
                    if (!e) return;
                    implantWeakness(state, member, e, 1);
                    e.resistance -= 0.20;
                    e.elemental_res = Object.fromEntries(
                        Object.entries(e.elemental_res).map(([el, val]) => [el, val - 0.20])
                    );
                    state.addLog({
                        type: 'event',
                        message: `E2: ${e.name} receives 1 Weakness implant and ∁E0% All-Type RES.`,
                    });
                });
            }
        },

        // ────────────────────────────────────────────────────────────────────────
        onTurnStart: (state, member) => {
            // A2: At turn start, if NO enemy currently has Qualitative Disclosure ↁE+30 Energy
            const anyQD = state.enemies.some(e => e && hasQD(state, e.instanceId));
            if (!anyQD) {
                gainEnergy(state, member, 30);
                state.addLog({ type: 'event', message: `A2 (no QD enemy): Anaxa gains +30 Energy.` });
            }
        },

        // ────────────────────────────────────────────────────────────────────────
        onBeforeAction: (state, member, action, target) => {
            if (!target) return;

            // ── QD bonus: +30% DMG to Qualitative Disclosure targets ─────────────
            if (hasQD(state, target.instanceId)) {
                member.buffs.dmg_boost += 30;
            }

            // ── A6: +4% DEF ignore per Weakness Type (max 7 types = 28%) ──────────
            member.buffs.def_ignore += Math.min(getWkCount(state, target.instanceId), 7) * 4;

            // ── E1 DEF shred carried from a previous hit ─────────────────────────
            if (member.eidolon >= 1 && target.activeDebuffs["anaxa_e1_def"]) {
                member.buffs.def_reduction += target.activeDebuffs["anaxa_e1_def"].value || 16;
            }

            // ── E4: Skill use ↁEATK +30% for 2 turns (stacks up to 2ÁE ──────────
            if (action.type === 'skill' && member.eidolon >= 4) {
                const existingStacks = getE4Stacks(state, member);
                if (existingStacks < 2) {
                    state.buffDurations[member.characterId] = state.buffDurations[member.characterId] || {};
                    // Use a unique key per stack to allow independent duration tracking
                    const stackKey = `anaxa_e4_stack_${Date.now() % 10000}`;
                    state.buffDurations[member.characterId][stackKey] = {
                        duration: 2, value: 30, stat: 'ATK%',
                    };
                }
                // Apply all currently active E4 stacks (including newly added one)
                member.buffs.atk_percent += getE4Stacks(state, member) * 30;
            }

            // ── Skill per-enemy DMG bonus: +20% per attackable enemy ─────────────
            if (action.type === 'skill') {
                const aliveCount = state.enemies.filter(e => e && e.hp > 0).length;
                member.buffs.dmg_boost += aliveCount * 20;
            }

            // All actions inflict debuffs (triggers Acheron / SW on-global-debuff hooks)
            action.inflictsDebuff = true;
        },

        // ────────────────────────────────────────────────────────────────────────
        onAfterAction: (state, member, action, target) => {
            if (!target) return;

            if (action.type === 'basic') {
                // Basic: 1 hit ↁE1 Weakness implant on main target
                implantWeakness(state, member, target, 1);

                // QD extra Skill trigger (basic path)
                if (hasQD(state, target.instanceId)) {
                    const triggerKey = `anaxa_qd_trigger_${state.currentActionId}`;
                    if (!state.stacks[triggerKey]) {
                        state.stacks[triggerKey] = 1;
                        state.stacks['anaxa_qd_pending_extra'] = 1;
                        state.addLog({
                            type: 'event',
                            message: `[Qualitative Disclosure] Basic triggers extra Skill on ${target.name}!`,
                        });
                        state.grantExtraTurn(member.characterId, '', { reason: 'Qualitative Disclosure' });
                    }
                }

                // A2: Basic ATK additionally regenerates 10 Energy
                gainEnergy(state, member, 30); // 20 base + 10 A2
            }

            if (action.type === 'skill') {
                // Resolve the 4 bounce hits (Hit[0] was already dealt by the normal action loop).
                // executeSkill with alreadyHitMain=true skips re-calculating Hit[0]'s damage
                // but still applies weakness implant and E1 DEF shred for it.
                executeSkill(state, member, action.multiplier, false, true);

                // E1: Recover 1 SP on the first Skill use of the battle
                if (member.eidolon >= 1 && !state.stacks['anaxa_e1_sp_done']) {
                    state.stacks['anaxa_e1_sp_done'] = 1;
                    state.skillPoints = Math.min(state.skillPoints + 1, 5);
                    state.addLog({ type: 'event', message: `E1: Recovered 1 Skill Point (first Skill use).` });
                }

                // Skill energy gain: 6
                gainEnergy(state, member, 6);
            }
        },

        // ────────────────────────────────────────────────────────────────────────
        onUlt: (state, member) => {
            const alive = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
            if (alive.length === 0) return;

            const ultMult = 1.60; // 160% ATK  ELv.10

            // ── Step 1: Inflict Sublimation on all enemies ────────────────────────
            // Sublimation applies all 7 Weakness Types and prevents the target from
            // taking action until the start of their turn.
            // Simulation: use the Freeze debuff for the turn-skip mechanic with 0 DoT damage
            // (attacker_break_effect = ∁E00 zeroes out the Freeze hit formula floor).
            alive.forEach(e => {
                // Implant all missing weakness types
                const prev = getWkCount(state, e.instanceId);
                const needed = 7 - prev;
                if (needed > 0) implantWeakness(state, member, e, needed);

                // Turn-skip via Freeze (Sublimation control effect)
                e.activeDebuffs["Freeze"] = {
                    duration: 1,
                    stat: "Sublimation",
                    value: 0,
                    attacker_level: member.level,
                    attacker_break_effect: -100,    // negates Freeze DoT damage to ~0
                    max_toughness_at_break: 1,      // minimise any residual damage
                };
                e.debuffCount = Object.keys(e.activeDebuffs).length;
                state.addLog({
                    type: 'event',
                    message: `${e.name} enters [Sublimation]: all Weakness Types implanted, action delayed.`,
                });
            });

            // ── Step 2: AoE Wind DMG (160% ATK) ───────────────────────────────────
            let totalUltDmg = 0;
            alive.forEach(e => {
                const buffs = buildHitBuffs(state, member, e);
                const tempMember = { ...member, buffs };
                const result = calculateHsrDamage({
                    character: tempMember, lightcone: member.lightcone,
                    enemy: e, ability_multiplier: ultMult, scaling_stat_id: ATK_ID,
                });
                totalUltDmg += result.expected_dmg;
                e.hp = Math.max(0, Math.floor(e.hp - result.expected_dmg));
                state.addLog({
                    type: 'event',
                    message: `Hit Ultimate on ${e.name} -> ${result.expected_dmg.toLocaleString()} DMG`,
                });

                // Toughness reduction (ignoring weakness check since Sublimation grants all types)
                if ((state as any).applyToughnessDamage) {
                    (state as any).applyToughnessDamage(e, 20, true);
                }
            });
            state.totalDamage += totalUltDmg;
            state.addLog({ type: 'event', message: `Ultimate total: ${totalUltDmg.toLocaleString()} DMG` });

            // ── Step 3: Energy  Econsumed (140), gain 5 from ult use ───────────────
            state.stacks[member.characterId] = 5;
        },

        // ────────────────────────────────────────────────────────────────────────
        onExtraTurn: (state, member) => {
            // All of Anaxa's extra turns are QD-triggered free Skills.
            // (Other extra-turn sources would use the simulator's default path instead.)
            const isQDTrigger = state.stacks['anaxa_qd_pending_extra'] === 1;
            state.stacks['anaxa_qd_pending_extra'] = 0;

            if (!isQDTrigger) {
                // Fallback: execute a Basic ATK (no SP cost, no QD chain)
                const alive = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
                if (alive.length === 0) return;
                const t = alive[0];
                const buffs = buildHitBuffs(state, member, t);
                const tempMember = { ...member, buffs };
                const result = calculateHsrDamage({
                    character: tempMember, lightcone: member.lightcone,
                    enemy: t, ability_multiplier: 1.00, scaling_stat_id: ATK_ID,
                });
                state.totalDamage += result.expected_dmg;
                t.hp = Math.max(0, Math.floor(t.hp - result.expected_dmg));
                state.addLog({
                    type: 'event',
                    message: `[Extra Turn] Basic on ${t.name} -> ${result.expected_dmg.toLocaleString()} DMG`,
                });
                implantWeakness(state, member, t, 1);
                gainEnergy(state, member, 30);
                return;
            }

            // ── QD free extra Skill: 5 hits at 70%, no SP cost, cannot chain again ─
            const alive = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
            if (alive.length === 0) return;

            const skillMult = 0.70; // Lv.10
            state.addLog({
                type: 'action',
                message: `[QD Extra Skill] Fractal, Exiles Fallacy (5 hits, free)`,
                actor: {
                    id: member.characterId,
                    name: member.name || 'Anaxa',
                    type: 'ally',
                },
            });

            // Execute all 5 hits (isQDExtra=true prevents chaining, alreadyHitMain=false)
            executeSkill(state, member, skillMult, true, false);

            // Energy from the extra Skill use
            gainEnergy(state, member, 6);
        },
    },

    special_modifiers: {
        // Energy managed manually via state.stacks[characterId]; the simulator uses this key
        // for the `state.stacks[id] >= energy_cost` ult-readiness check.
        energy_type: 'NONE',
        energy_cost: 140,

        eidolon_level_boosts: (eidolon: number) => ({
            ...(eidolon >= 3 ? { ultimate: 2, basic: 1 } : {}),
            ...(eidolon >= 5 ? { skill: 2, talent: 2 } : {}),
        }),

        // Minor traces
        // +10% HP is not modelled in the damage formula; +22.4% Wind DMG = pure DMG boost for Anaxa
        stat_boosts: () => ({
            crit_rate: 12,    // +12% CRIT Rate
            dmg_boost: 22.4,  // +22.4% Wind DMG Boost (Anaxa deals only Wind DMG)
        }),
    },
};
