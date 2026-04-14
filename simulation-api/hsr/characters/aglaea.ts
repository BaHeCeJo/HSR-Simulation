/**
 * @character Aglaea
 * @role Main DPS / Speed Buffer (Remembrance)
 * @core_mechanic Memosprite Garmentmaker generates SPD Boost stacks. Supreme Stance (Ultimate)
 *   enhances Aglaea's Basic ATK to a Joint ATK with Garmentmaker, and transfers SPD Boost to Aglaea.
 */

import type { CharacterKit, SimState, TeamMember, SimEnemy, Action, SummonUnit } from "../types.js";
import { calculateHsrDamage } from "../formulas.js";

// IDs from JSON/Mapping
export const AGLAEA_ID = 'e1e8adcb-ba8f-4c38-adec-cc5c9bfe09e1';
export const GARMENTMAKER_ID = 'b9d5f7a3-c4e6-4901-9bcd-f01234567890';
const COUNTDOWN_ID = 'c0e6a8b4-d5f7-4012-abcd-ef1234567890';

const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const CHAR_SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';

// State helpers
const energyKey = (id: string) => id;
const stanceKey = (id: string) => `agl_stance_${id}`;
const origSpdKey = (id: string) => `agl_orig_spd_${id}`;

const isSupremeStance = (state: SimState, id: string) => (state.stacks[stanceKey(id)] || 0) === 1;
const getSpdStacks = (state: SimState) => state.stacks['gm_spd_stacks'] || 0;
const getMaxSpdStacks = (eidolon: number) => eidolon >= 4 ? 7 : 6;

function getAglaea(state: SimState): TeamMember | undefined {
    return state.team.find(m => m.characterId === AGLAEA_ID);
}

function getGarmentmaker(state: SimState): SummonUnit | undefined {
    return state.summons.find(s => s.id === GARMENTMAKER_ID);
}

function computeGarmSPD(state: SimState, aglaeaBaseSPD: number): number {
    return aglaeaBaseSPD * 0.35 + getSpdStacks(state) * 55;
}

function addEnergy(state: SimState, aglaea: TeamMember, amount: number) {
    const k = energyKey(aglaea.characterId);
    state.stacks[k] = Math.min((state.stacks[k] || 0) + amount, 350);
}

function applySeamStitch(state: SimState, target: SimEnemy) {
    const prev = String(state.stacks['seam_stitch_target'] || '');
    if (prev && prev !== target.instanceId) {
        const prevEnemy = state.enemies.find((e): e is SimEnemy => !!e && e.instanceId === prev);
        if (prevEnemy) {
            delete prevEnemy.activeDebuffs['seam_stitch'];
        }
    }
    state.stacks['seam_stitch_target'] = target.instanceId as any;
    target.activeDebuffs['seam_stitch'] = { duration: 999, stat: 'Seam Stitch' };
}

function syncGarmSPD(state: SimState, aglaeaOrigSPD: number) {
    const gm = getGarmentmaker(state);
    if (gm) gm.spd = computeGarmSPD(state, aglaeaOrigSPD);
}

function syncAglaeaSPD(state: SimState, aglaea: TeamMember) {
    const origSPD = state.stacks[origSpdKey(aglaea.characterId)] || (aglaea.base_stats[CHAR_SPD_ID] || 100);
    const stacks = getSpdStacks(state);
    aglaea.base_stats[CHAR_SPD_ID] = origSPD * (1 + stacks * 0.15);
}

function dealSeamStitchDmg(state: SimState, target: SimEnemy, aglaea: TeamMember) {
    if (!target.activeDebuffs['seam_stitch']) return;

    const result = calculateHsrDamage({
        character: aglaea,
        lightcone: aglaea.lightcone,
        enemy: target,
        ability_multiplier: 0.30,
        scaling_stat_id: ATK_ID
    });
    state.totalDamage += result.expected_dmg;
    target.hp = Math.max(0, Math.floor(target.hp - result.expected_dmg));
    state.addLog({ type: 'event', message: `Seam Stitch bonus on ${target.name} -> ${result.expected_dmg.toLocaleString()} DMG` });

    addEnergy(state, aglaea, 10);
}

function applyToughnessDmgHelper(state: SimState, attacker: TeamMember | SummonUnit, target: SimEnemy, amount: number) {
    if ((state as any).applyToughnessDamage) {
        (state as any).applyToughnessDamage(target, amount, false);
    }
}

export const Aglaea: CharacterKit = {
    id: AGLAEA_ID,
    name: "Aglaea",
    path: "Remembrance",
    element: "Lightning",
    slot_names: {
        basic: "Thorned Nectar",
        skill: "Rise, Exalted Renown",
        ultimate: "Dance, Destined Weaveress",
        talent: "Rosy-Fingered",
    },
    abilities: {
        basic: {
            default_multiplier: 1.0,
            stat_id: ATK_ID,
            targetType: 'SingleTarget',
            toughness_damage: 10
        },
        skill: { default_multiplier: 0, stat_id: ATK_ID, targetType: 'Support' },
        ultimate: { default_multiplier: 0, stat_id: ATK_ID, targetType: 'Enhance' },
        talent: {
            seam_stitch_dmg: { default_multiplier: 0.30, stat_id: ATK_ID, targetType: 'SingleTarget' }
        }
    },
    hooks: {
        onBattleStart: (state, member) => {
            state.stacks[energyKey(member.characterId)] = 175; // A6 passive
            state.stacks[stanceKey(member.characterId)] = 0;
            state.stacks['gm_spd_stacks'] = 0;
            state.addLog({ type: 'event', message: `A6: Aglaea begins with 175 Energy.` });
        },
        onTurnStart: (state, member) => {
            if (isSupremeStance(state, member.characterId)) {
                syncAglaeaSPD(state, member);
            }
        },
        onBeforeAction: (state, member, action, target) => {
            const inStance = isSupremeStance(state, member.characterId);

            if (member.eidolon >= 2) {
                const k = `agl_e2_${member.characterId}`;
                state.stacks[k] = Math.min((state.stacks[k] || 0) + 1, 3);
                member.buffs.def_ignore += (state.stacks[k] || 0) * 14;
            }

            if (inStance && action.type === 'basic') {
                action.multiplier = 0;
                action.toughness_damage = 0;
                state.stacks['agl_do_enhanced_basic'] = 1;
            }
        },
        onAfterAction: (state, member, action, target) => {
            if (action.type === 'skill') {
                const existingGM = getGarmentmaker(state);
                if (!existingGM) {
                    const aglaeaBaseSPD = state.stacks[origSpdKey(member.characterId)] || (member.base_stats[CHAR_SPD_ID] || 100);
                    const garmSPD = computeGarmSPD(state, aglaeaBaseSPD);
                    state.summonUnit({
                        id: GARMENTMAKER_ID,
                        instanceId: GARMENTMAKER_ID,
                        name: "Garmentmaker",
                        kind: "memosprite",
                        masterId: AGLAEA_ID,
                        element: "Lightning",
                        level: member.level,
                        spd: garmSPD,
                        hp: Math.floor(member.max_hp * 0.66) + 720,
                        max_hp: Math.floor(member.max_hp * 0.66) + 720,
                        shield: 0,
                        aggroValue: 100,
                        canBeTargetedByEnemies: true,
                        canBeTargetedByAllies: true,
                        base_stats: { ...member.base_stats, [CHAR_SPD_ID]: garmSPD },
                        buffs: { ...member.buffs },
                        activeBuffs: {},
                        activeDebuffs: {},
                        activeShields: [],
                        lightcone: member.lightcone
                    });
                }
                addEnergy(state, member, 20);
                return;
            }

            if (!target) return;

            if (action.type === 'basic' && isSupremeStance(state, member.characterId) && state.stacks['agl_do_enhanced_basic']) {
                state.stacks['agl_do_enhanced_basic'] = 0;
                
                // Enhanced Basic Logic (Simplified for port)
                dealSeamStitchDmg(state, target, member);
                
                const hit = (mult: number, t: SimEnemy, label: string) => {
                    const res = calculateHsrDamage({ character: member, lightcone: member.lightcone, enemy: t, ability_multiplier: mult, scaling_stat_id: ATK_ID });
                    state.totalDamage += res.expected_dmg;
                    t.hp = Math.max(0, Math.floor(t.hp - res.expected_dmg));
                    state.addLog({ type: 'event', message: `${label} on ${t.name} -> ${res.expected_dmg.toLocaleString()} DMG` });
                };

                hit(2.0, target, "Joint ATK [Aglaea]");
                applyToughnessDmgHelper(state, member, target, 20);
                applySeamStitch(state, target);
                addEnergy(state, member, 20);
            } else if (action.type === 'basic') {
                dealSeamStitchDmg(state, target, member);
                applySeamStitch(state, target);
                addEnergy(state, member, 20);
            }
        },
        onUlt: (state, member) => {
            state.stacks[energyKey(member.characterId)] = 5;
            state.stacks[stanceKey(member.characterId)] = 1;
            state.stacks[origSpdKey(member.characterId)] = member.base_stats[CHAR_SPD_ID] || 100;
            
            syncAglaeaSPD(state, member);
            state.addLog({ type: 'event', message: `Supreme Stance active! Aglaea SPD boosted.` });
        }
    },
    special_modifiers: {
        energy_type: "ENERGY",
        energy_cost: 350,
        stat_boosts: () => ({ crit_rate: 12 }),
        eidolon_level_boosts: (eidolon: number) => ({
            ...(eidolon >= 3 ? { skill: 2, basic: 1 } : {}),
            ...(eidolon >= 5 ? { ultimate: 2, talent: 2 } : {})
        })
    }
};
