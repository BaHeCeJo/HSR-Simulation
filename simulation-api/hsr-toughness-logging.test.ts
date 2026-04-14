import { describe, it, expect } from 'vitest';
import { runCombatSimulation } from './hsr/simulator';
import { Acheron } from './hsr/characters/acheron';
import { TeamMember, SimEnemy } from './hsr/types';
import { Antibaryon } from './hsr/enemies/antibaryon';

describe('HSR Toughness and Break Logging', () => {
  it('should log toughness damage and weakness breaks for both allies and enemies', () => {
    const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
    const SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';
    const HP_ID = '7383172e-f828-4298-a8cf-887d50ff4a28';
    const DEF_ID = '73868117-3df2-470d-945a-e389f9f04200';

    const ENEMY_ATK_ID = '7761c316-9c6b-4610-aa72-afcb80aeb1e9';
    const ENEMY_HP_ID = 'dab1d58a-5e35-470a-a2d4-1bdddf3019a0';
    const ENEMY_SPD_ID = 'b0bfd27b-0a5f-4329-a280-dc1c998446cb';

    const team: TeamMember[] = [
      {
        characterId: Acheron.id,
        level: 80,
        eidolon: 0,
        hp: 3500,
        max_hp: 3500,
        shield: 0,
        toughness: 100,
        max_toughness: 100,
        is_broken: false,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: { [ATK_ID]: 1200, [SPD_ID]: 101, [HP_ID]: 3500, [DEF_ID]: 600 },
        buffs: { atk_percent: 0, crit_rate: 5, crit_dmg: 50, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: { base_stats: { [ATK_ID]: 635, [HP_ID]: 1000, [DEF_ID]: 400 }, scaling: 1.0 }
      }
    ];

    const enemy: SimEnemy = {
      id: Antibaryon.id,
      instanceId: "enemy_01",
      name: "Antibaryon",
      level: 90,
      hp: 100000, // High HP to survive multiple turns
      max_hp: 100000,
      toughness: 10,
      max_toughness: 10,
      weaknesses: ["Lightning"],
      resistance: 0.2,
      is_broken: false,
      vulnerability: 0,
      dmg_reduction: 0,
      weaken: 0,
      debuffCount: 0,
      activeDebuffs: {},
      activeBuffs: {},
      base_stats: { [ENEMY_SPD_ID]: 150, [ENEMY_ATK_ID]: 2000, [ENEMY_HP_ID]: 100000 }
    };

    const report = runCombatSimulation(team, enemy, 5);
    
    // Check if toughness damage is logged for enemy
    const enemyToughnessLogs = report.logs.filter(l => 
        l.message.includes("Reduced Antibaryon's Toughness") || 
        (l.subEntries && l.subEntries.some(s => s.includes("Reduced Antibaryon's Toughness")))
    );
    expect(enemyToughnessLogs.length).toBeGreaterThan(0);
    
    // Check if toughness damage is logged for ally
    const allyToughnessLogs = report.logs.filter(l => 
        l.message.includes("Reduced Acheron's Toughness") || 
        (l.subEntries && l.subEntries.some(s => s.includes("Reduced Acheron's Toughness")))
    );
    expect(allyToughnessLogs.length).toBeGreaterThan(0);

    // Check for weakness breaks
    const breakLogs = report.logs.filter(l => 
        l.message.includes("[WEAKNESS BREAK]") || 
        (l.subEntries && l.subEntries.some(s => s.includes("[WEAKNESS BREAK]")))
    );
    expect(breakLogs.length).toBeGreaterThan(0);

    // Output logs for visual verification
    console.log("=== TOUGHNESS LOGGING TEST ===");
    report.logs.forEach(log => {
        const actor = log.actor ? `${log.actor.name}: ` : "";
        console.log(`[${log.av}] ${actor}${log.message}`);
        if (log.subEntries) {
            log.subEntries.forEach(sub => console.log(`  ━E${sub}`));
        }
    });
  });
});
