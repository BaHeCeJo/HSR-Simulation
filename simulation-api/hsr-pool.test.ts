import { describe, it, expect } from 'vitest';
import { runCombatSimulation } from './hsr/simulator';
import { Acheron } from './hsr/characters/acheron';
import { Baryon } from './hsr/enemies/baryon';
import { TeamMember, SimEnemy } from './hsr/types';

describe('HSR Enemy Pool Bug Reproduction', () => {
  it('should put overflow enemies into the pool when an array is passed', () => {
    const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
    const SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';
    const HP_ID = '7383172e-f828-4298-a8cf-887d50ff4a28';
    const DEF_ID = '73868117-3df2-470d-945a-e389f9f04200';

    const ENEMY_HP_ID = 'dab1d58a-5e35-470a-a2d4-1bdddf3019a0';
    const ENEMY_SPD_ID = 'b0bfd27b-0a5f-4329-a280-dc1c998446cb';

    const team: TeamMember[] = [
      {
        characterId: Acheron.id,
        level: 80,
        eidolon: 6,
        hp: 10000,
        max_hp: 10000,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: { [ATK_ID]: 2000, [SPD_ID]: 150, [HP_ID]: 10000, [DEF_ID]: 1000 },
        buffs: { atk_percent: 0, crit_rate: 100, crit_dmg: 100, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: { base_stats: { [ATK_ID]: 500 }, scaling: 1.0 }
      }
    ];

    const buildEnemy = (instanceId: string): SimEnemy => ({
      id: Baryon.id,
      instanceId,
      name: "Baryon",
      level: 80,
      hp: 1000, // Small HP to die quickly
      max_hp: 1000,
      resistance: 0.2,
      is_broken: false,
      vulnerability: 0,
      dmg_reduction: 0,
      weaken: 0,
      debuffCount: 0,
      activeDebuffs: {},
      activeBuffs: {},
      base_stats: { [ENEMY_SPD_ID]: 100, [ENEMY_HP_ID]: 1000 }
    });

    // 7 enemies total
    const enemies = [
      buildEnemy("e1"),
      buildEnemy("e2"),
      buildEnemy("e3"),
      buildEnemy("e4"),
      buildEnemy("e5"),
      buildEnemy("e6"),
      buildEnemy("e7"),
    ];

    const report = runCombatSimulation(team, enemies, 1);
    
    console.log("=== POOL BUG REPRODUCTION LOGS ===");
    report.logs.forEach(log => console.log(log));
    
    const logs = report.logs.map(l => l.message).join('\n');
    
    // Check if e6 and e7 ever entered the field
    expect(logs).toContain('Baryon (e6) enters the field');
    expect(logs).toContain('Baryon (e7) enters the field');
  });
});
