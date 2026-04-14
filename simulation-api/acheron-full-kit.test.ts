import { it, expect, describe } from "vitest";
import { runCombatSimulation } from "./hsr/simulator";
import { TeamMember, SimEnemy } from "./hsr/types";
import { Acheron } from "./hsr/characters/acheron";
import { Jiaoqiu } from "./hsr/characters/jiaoqiu";

const CHAR_HP_ID = '7383172e-f828-4298-a8cf-887d50ff4a28';
const CHAR_SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';
const ENEMY_HP_ID = 'dab1d58a-5e35-470a-a2d4-1bdddf3019a0';
const ENEMY_SPD_ID = 'b0bfd27b-0a5f-4329-a280-dc1c998446cb';
const ENEMY_ATK_ID = '7761c316-9c6b-4610-aa72-afcb80aeb1e9';
const ENEMY_TOUGHNESS_ID = '50ff424d-9428-46e2-8f3e-8968dacbb6bd';

const hasLog = (logs: any[], msg: string) => logs.some(l => 
    l.message.includes(msg) || (l.subEntries && l.subEntries.some((s: string) => s.includes(msg)))
);

describe("Acheron Full Kit Validation", () => {
  it("should handle Slashed Dream over-capping into Quadrivalent Ascendance and refunding post-Ult", () => {
    const acheron: TeamMember = {
      characterId: Acheron.id,
      name: "Acheron",
      element: "Lightning",
      level: 80,
      eidolon: 0,
      hp: 3500,
      max_hp: 3500,
      shield: 0,
      abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
      base_stats: { [CHAR_HP_ID]: 3500, [CHAR_SPD_ID]: 101 },
      buffs: { atk_percent: 0, crit_rate: 0, crit_dmg: 0, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
      activeBuffs: {},
      activeDebuffs: {},
      lightcone: { base_stats: {}, scaling: 1 }
    };

    const jiaoqiu: TeamMember = {
        characterId: Jiaoqiu.id,
        name: "Jiaoqiu",
        element: "Fire",
        level: 80,
        eidolon: 0,
        hp: 3000,
        max_hp: 3000,
        shield: 0,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: { [CHAR_HP_ID]: 3000, [CHAR_SPD_ID]: 160 },
        buffs: { atk_percent: 0, crit_rate: 0, crit_dmg: 0, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: { base_stats: {}, scaling: 1 }
    };

    const enemy: SimEnemy = {
      id: "enemy_01",
      instanceId: "enemy_01",
      name: "Boss",
      level: 80,
      hp: 10000000,
      max_hp: 10000000,
      toughness: 300,
      max_toughness: 300,
      weaknesses: ["Lightning", "Fire"],
      resistance: 0.2,
      elemental_res: {},
      is_broken: false,
      vulnerability: 0,
      dmg_reduction: 0,
      weaken: 0,
      debuffCount: 0,
      activeDebuffs: {},
      activeBuffs: {},
      base_stats: { [ENEMY_HP_ID]: 10000000, [ENEMY_SPD_ID]: 50, [ENEMY_ATK_ID]: 100, [ENEMY_TOUGHNESS_ID]: 300 }
    };

    const report = runCombatSimulation([acheron, jiaoqiu], enemy, 10);

    // Verify QA refund in logs
    expect(hasLog(report.logs, "Quadrivalent Ascendance")).toBe(true);
    
    // Check if Crimson Knots were removed during Ult
    expect(hasLog(report.logs, "Removed")).toBe(true);
  });

  it("should transfer Crimson Knots when an enemy is defeated", () => {
    const acheron: TeamMember = {
      characterId: Acheron.id,
      name: "Acheron",
      element: "Lightning",
      level: 80,
      eidolon: 0,
      hp: 3500,
      max_hp: 3500,
      shield: 0,
      abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
      base_stats: { [CHAR_HP_ID]: 3500, [CHAR_SPD_ID]: 10 }, // Slow so Jiaoqiu acts first
      buffs: { atk_percent: 0, crit_rate: 0, crit_dmg: 0, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
      activeBuffs: {},
      activeDebuffs: {},
      lightcone: { base_stats: {}, scaling: 1 }
    };

    const jiaoqiu: TeamMember = {
        characterId: Jiaoqiu.id,
        name: "Jiaoqiu",
        element: "Fire",
        level: 80,
        eidolon: 0,
        hp: 3000,
        max_hp: 3000,
        shield: 0,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: { [CHAR_HP_ID]: 3000, [CHAR_SPD_ID]: 200 }, // Fast
        buffs: { atk_percent: 0, crit_rate: 0, crit_dmg: 0, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: { base_stats: {}, scaling: 1 }
    };

    const enemy1: SimEnemy = {
      id: "enemy_1",
      instanceId: "enemy_1",
      name: "Minion 1",
      level: 80,
      hp: 1000, 
      max_hp: 1000,
      toughness: 30,
      max_toughness: 30,
      weaknesses: ["Fire"],
      resistance: 0.2,
      elemental_res: {},
      is_broken: false,
      vulnerability: 0,
      dmg_reduction: 0,
      weaken: 0,
      debuffCount: 0,
      activeDebuffs: {},
      activeBuffs: {},
      base_stats: { [ENEMY_HP_ID]: 1000, [ENEMY_SPD_ID]: 5, [ENEMY_ATK_ID]: 100 }
    };

    const enemy2: SimEnemy = {
        id: "enemy_2",
        instanceId: "enemy_2",
        name: "Minion 2",
        level: 80,
        hp: 1000000,
        max_hp: 1000000,
        toughness: 30,
        max_toughness: 30,
        weaknesses: ["Fire"],
        resistance: 0.2,
        elemental_res: {},
        is_broken: false,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0,
        debuffCount: 0,
        activeDebuffs: {},
        activeBuffs: {},
        base_stats: { [ENEMY_HP_ID]: 1000000, [ENEMY_SPD_ID]: 5, [ENEMY_ATK_ID]: 100 }
    };

    // Use a longer simulation to ensure Acheron eventually acts and kills the weakened enemy1
    const report = runCombatSimulation([acheron, jiaoqiu], [enemy1, enemy2], 5);

    // DEBUG
    // console.log("=== ACHERON TRANSFER TEST LOGS ===");
    // report.logs.forEach(l => {
    //     console.log(`[${l.av}] ${l.message}`);
    //     l.subEntries?.forEach(s => console.log(`  - ${s}`));
    // });

    expect(hasLog(report.logs, "Transferred")).toBe(true);
    expect(hasLog(report.logs, "Crimson Knots")).toBe(true);
  });
});
