import { describe, it } from 'vitest';
import { runCombatSimulation } from './hsr/simulator';
import { Acheron } from './hsr/characters/acheron';
import { Pela } from './hsr/characters/pela';
import { TeamMember, SimEnemy, Wave } from './hsr/types';
import { Antibaryon } from './hsr/enemies/antibaryon';
import { Baryon } from './hsr/enemies/baryon';

describe('HSR Wave Combat Simulation', () => {
  it('should run a simulation with multiple waves and enemy pools', () => {
    // Stat IDs from HSR_ID_MAPPING.md
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
        hp: 100000,
        max_hp: 100000,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: { [ATK_ID]: 5000, [SPD_ID]: 200, [HP_ID]: 100000, [DEF_ID]: 1000 },
        buffs: { atk_percent: 100, crit_rate: 100, crit_dmg: 500, dmg_boost: 200, def_ignore: 100, extra_multiplier: 0, extra_dmg: 0, res_pen: 100 },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: { base_stats: { [ATK_ID]: 1000, [HP_ID]: 2000, [DEF_ID]: 600 }, scaling: 1.0 }
      }
    ];

    const buildEnemy = (id: string, instanceId: string, hp: number = 2000): SimEnemy => ({
      id,
      instanceId,
      name: id === Antibaryon.id ? "Antibaryon" : "Baryon",
      level: 80,
      hp,
      max_hp: hp,
      resistance: 0.2,
      is_broken: false,
      vulnerability: 0,
      dmg_reduction: 0,
      weaken: 0,
      debuffCount: 0,
      activeDebuffs: {},
      activeBuffs: {},
      base_stats: { [ENEMY_SPD_ID]: 110, [ENEMY_HP_ID]: hp }
    });

    const waves: Wave[] = [
      {
        initialEnemies: [
          buildEnemy(Baryon.id, "w1_e1"),
          null,
          buildEnemy(Antibaryon.id, "w1_e2"),
          null,
          buildEnemy(Baryon.id, "w1_e3"),
        ],
        enemyPool: [
          buildEnemy(Baryon.id, "w1_p1"),
          buildEnemy(Antibaryon.id, "w1_p2"),
        ]
      },
      {
        initialEnemies: [
          buildEnemy(Antibaryon.id, "w2_e1", 5000),
          buildEnemy(Antibaryon.id, "w2_e2", 5000),
        ],
        enemyPool: []
      }
    ];

    const report = runCombatSimulation(team, waves, 5);
    
    console.log("=== HSR WAVE COMBAT SIMULATION LOGS ===");
    report.logs.forEach(log => console.log(log));
    console.log("========================================");
  });
});
