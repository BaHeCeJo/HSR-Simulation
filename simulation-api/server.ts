import Fastify from 'fastify';
import cors from '@fastify/cors';
import { runCombatSimulation } from './hsr/simulator.js';
import { HSR_CHARACTER_KITS } from './hsr/registry.js';
import type { TeamMember, SimEnemy, Wave, LogEntry } from './hsr/types.js';

const fastify = Fastify({ logger: true });

// Definitive UUIDs from HSR_ID_MAPPING.md
const CHAR_HP_ID = '7383172e-f828-4298-a8cf-887d50ff4a28';
const CHAR_SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';
const ENEMY_HP_ID = 'dab1d58a-5e35-470a-a2d4-1bdddf3019a0';
const ENEMY_SPD_ID = 'b0bfd27b-0a5f-4329-a280-dc1c998446cb';
const ENEMY_TOUGHNESS_ID = '50ff424d-9428-46e2-8f3e-8968dacbb6bd';
const LC_ATK_ID = '8e5af9db-3079-49ef-90c3-747b4ea00025';

interface StatValue {
    value: number;
    name: string;
}

interface Character {
    character_id: string;
    name: string;
    level: number;
    eidolon: number;
    attribute: string;
    path: string;
    basic_stats: Record<string, StatValue>;
    advanced_stats: Record<string, StatValue>;
    abilities: any[];
}

interface Lightcone {
    lightcone_id: string;
    name: string;
    level: number;
    superimposition: number;
    path: string;
    basic_stats: Record<string, StatValue>;
    advanced_stats: Record<string, StatValue>;
}

interface Enemy {
    id: string;
    instance_id: string;
    name: string;
    level: number;
    basic_stats: Record<string, StatValue>;
    advanced_stats: Record<string, StatValue>;
    resistances: Record<string, number>;
    weaknesses: string[];
    tier: string;
}

function mapToTeamMember(char: Character, lc: Lightcone | null): TeamMember {
    const kit = HSR_CHARACTER_KITS[char.character_id];
    
    const base_stats: Record<string, number> = {};
    if (char.basic_stats) {
        for (const [id, val] of Object.entries(char.basic_stats)) {
            base_stats[id] = val.value;
        }
    }
    if (char.advanced_stats) {
        for (const [id, val] of Object.entries(char.advanced_stats)) {
            base_stats[id] = val.value;
        }
    }

    const lc_stats: Record<string, number> = {};
    if (lc) {
        if (lc.basic_stats) {
            for (const [id, val] of Object.entries(lc.basic_stats)) {
                lc_stats[id] = val.value;
            }
        }
        if (lc.advanced_stats) {
            for (const [id, val] of Object.entries(lc.advanced_stats)) {
                lc_stats[id] = val.value;
            }
        }
    }

    const hp = base_stats[CHAR_HP_ID] || 3000;

    // Map ability levels from payload if possible
    const abilityLevels = { basic: 6, skill: 10, ultimate: 10, talent: 10 };
    if (char.abilities && kit?.slot_names) {
        char.abilities.forEach(ability => {
            if (ability.name === kit.slot_names.basic) abilityLevels.basic = ability.level;
            else if (ability.name === kit.slot_names.skill) abilityLevels.skill = ability.level;
            else if (ability.name === kit.slot_names.ultimate) abilityLevels.ultimate = ability.level;
            else if (ability.name === kit.slot_names.talent) abilityLevels.talent = ability.level;
        });
    }

    return {
        characterId: char.character_id,
        name: char.name || kit?.name,
        element: char.attribute || kit?.element || "Physical",
        path: char.path || kit?.path,
        aggroModifier: 0,
        level: char.level,
        eidolon: char.eidolon || 0,
        hp,
        max_hp: hp,
        shield: 0,
        is_broken: false,
        toughness: 100,
        max_toughness: 100,
        abilityLevels,
        databaseAbilities: char.abilities,
        base_stats,
        buffs: {
            atk_percent: 0,
            crit_rate: 5,
            crit_dmg: 50,
            dmg_boost: 0,
            def_ignore: 0,
            def_reduction: 0,
            extra_multiplier: 0,
            extra_dmg: 0,
            res_pen: 0,
            weaken: 0,
            break_efficiency: 0
        },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: {
            base_stats: lc_stats,
            scaling: 1.0
        }
    };
}

function mapToSimEnemy(enemy: Enemy): SimEnemy {
    const base_stats: Record<string, number> = {};
    if (enemy.basic_stats) {
        for (const [id, val] of Object.entries(enemy.basic_stats)) {
            base_stats[id] = val.value;
        }
    }
    if (enemy.advanced_stats) {
        for (const [id, val] of Object.entries(enemy.advanced_stats)) {
            base_stats[id] = val.value;
        }
    }

    const hp = base_stats[ENEMY_HP_ID] || 10000;
    const toughness = base_stats[ENEMY_TOUGHNESS_ID] || 100;

    return {
        id: enemy.id,
        instanceId: enemy.instance_id,
        name: enemy.name,
        level: enemy.level,
        hp: hp,
        max_hp: hp,
        toughness: toughness,
        max_toughness: toughness,
        extra_toughness_bars: [],
        max_extra_toughness_bars: [],
        weaknesses: enemy.weaknesses && enemy.weaknesses.length > 0 ? enemy.weaknesses : [],
        resistance: 0.2,
        elemental_res: enemy.resistances || {},
        is_broken: false,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0,
        debuffCount: 0,
        tier: enemy.tier || 'normal',
        activeDebuffs: {},
        activeBuffs: {},
        base_stats
    };
}

function* combinationsGen<T>(array: T[], n: number): Generator<T[]> {
    if (n === 0) { yield []; return; }
    for (let i = 0; i <= array.length - n; i++) {
        for (const tail of combinationsGen(array.slice(i + 1), n - 1)) {
            yield [array[i], ...tail];
        }
    }
}

function* permutationsGen<T>(array: T[], n: number): Generator<(T | null)[]> {
    if (n === 0) { yield []; return; }
    for (let i = 0; i < array.length; i++) {
        const remaining = [...array.slice(0, i), ...array.slice(i + 1)];
        for (const tail of permutationsGen(remaining, n - 1)) {
            yield [array[i], ...tail];
        }
    }
    if (array.length < n) {
        for (const tail of permutationsGen(array, n - 1)) {
            yield [null, ...tail];
        }
    }
}

function formatLogs(logs: LogEntry[]): string[] {
    return logs.map(l => {
        const timestamp = `[AV ${l.av.toFixed(1)}]`;
        let msg = `${timestamp} ${l.message}`;
        if (l.subEntries) {
            l.subEntries.forEach(se => {
                msg += `\n  - ${se}`;
            });
        }
        return msg;
    }).flatMap(msg => msg.split('\n'));
}

fastify.register(cors);

fastify.post('/optimize', async (request, _reply) => {
    const body: any = request.body;
    const payload = body.payload;
    const charPool = payload.character_pool as Character[];
    const lcPool = payload.lightcone_pool as Lightcone[];
    const wavesData = payload.waves;
    const settings = payload.settings;

    const teamSize = Math.min(charPool.length, 4);
    const lcSlice = lcPool.slice(0, 4);

    let bestTeamInfo: { char: Character, lc: Lightcone | null }[] = [];
    let bestTotalDamage = -1;
    let bestLogs: string[] = [];

    // Map waves
    const waves: Wave[] = wavesData.map((w: any) => {
        const initialEnemies: (SimEnemy | null)[] = [null, null, null, null, null];
        const enemies = (w.enemies || []).filter((e: any) => e !== null);
        enemies.forEach((e: any, i: number) => {
            if (i < 5) initialEnemies[i] = mapToSimEnemy(e);
        });
        return {
            initialEnemies,
            enemyPool: []
        };
    });

    let bestIsDefeated = true;
    let bestCycles = 999;
    let simulationsCount = 0;
    let bestAssignment: (Lightcone | null)[] = [];

    // Sweep all combinations without generating logs to minimise memory pressure.
    for (const teamChars of combinationsGen(charPool, teamSize)) {
        for (const assignment of permutationsGen(lcSlice, teamSize)) {
            simulationsCount++;
            const teamMembers = teamChars.map((char, i) => mapToTeamMember(char, assignment[i]));

            const report = runCombatSimulation(teamMembers, waves, settings.max_cycles, { skipLogs: true });

            // Priority:
            // 1. Survived (isDefeated = false) > Defeated
            // 2. Faster clear (lower cyclesTaken)
            // 3. Higher damage (if cycles are the same)
            const isBetter =
                (bestTotalDamage === -1) ||
                (bestIsDefeated && !report.isDefeated) ||
                (bestIsDefeated === report.isDefeated && report.cyclesTaken < bestCycles) ||
                (bestIsDefeated === report.isDefeated && report.cyclesTaken === bestCycles && report.totalDamage > bestTotalDamage);

            if (isBetter) {
                bestTotalDamage = report.totalDamage;
                bestIsDefeated = report.isDefeated || false;
                bestCycles = report.cyclesTaken;
                bestTeamInfo = teamChars.map((char, i) => ({ char, lc: assignment[i] }));
                bestAssignment = [...assignment];
            }
        }
    }

    // Re-run the winner with logs enabled to produce the detailed report.
    if (bestTeamInfo.length > 0) {
        const bestMembers = bestTeamInfo.map((t, i) => mapToTeamMember(t.char, bestAssignment[i]));
        const bestReport = runCombatSimulation(bestMembers, waves, settings.max_cycles);
        bestLogs = formatLogs(bestReport.logs);
    }

    return {
        bestTeam: bestTeamInfo.map(t => `${t.char.name} (${t.lc ? t.lc.name : 'None'})`),
        totalDamage: Math.floor(bestTotalDamage),
        cycles: bestCycles,
        logs: bestLogs,
        simulationsCount: simulationsCount,
        isDefeated: bestIsDefeated
    };
});

const start = async () => {
    try {
        await fastify.listen({ port: 3000, host: '0.0.0.0' });
        console.log('Server is running on http://localhost:3000');
    } catch (err) {
        fastify.log.error(err);
        process.exit(1);
    }
};

start();
