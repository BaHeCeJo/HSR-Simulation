import { describe, it } from 'vitest';
import { runCombatSimulation } from './hsr/simulator';
import { Acheron } from './hsr/characters/acheron';
import { Pela } from './hsr/characters/pela';
import { SilverWolf } from './hsr/characters/silver-wolf';
import { TeamMember, SimEnemy } from './hsr/types';

import { Antibaryon } from './hsr/enemies/antibaryon';

describe('HSR Detailed Combat Simulation', () => {
  it('should run a 3-cycle simulation with Enemy turns and status tracking', () => {
    // Stat IDs from HSR_ID_MAPPING.md
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
        eidolon: 2,
        hp: 3500,
        max_hp: 3500,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: { [ATK_ID]: 1200, [SPD_ID]: 101, [HP_ID]: 3500, [DEF_ID]: 600 },
        buffs: { atk_percent: 0, crit_rate: 5, crit_dmg: 50, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: { base_stats: { [ATK_ID]: 635, [HP_ID]: 1000, [DEF_ID]: 400 }, scaling: 1.0 }
      },
      {
        characterId: Pela.id,
        level: 80,
        eidolon: 6,
        hp: 3000,
        max_hp: 3000,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: { [ATK_ID]: 1000, [SPD_ID]: 134, [HP_ID]: 3000, [DEF_ID]: 500 },
        buffs: { atk_percent: 0, crit_rate: 5, crit_dmg: 50, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: { base_stats: { [ATK_ID]: 476, [HP_ID]: 800, [DEF_ID]: 300 }, scaling: 1.0 }
      },
      {
        characterId: SilverWolf.id,
        level: 80,
        eidolon: 0,
        hp: 3200,
        max_hp: 3200,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: { [ATK_ID]: 1100, [SPD_ID]: 145, [HP_ID]: 3200, [DEF_ID]: 550 },
        buffs: { atk_percent: 0, crit_rate: 5, crit_dmg: 50, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: { base_stats: { [ATK_ID]: 582, [HP_ID]: 900, [DEF_ID]: 350 }, scaling: 1.0 }
      }
    ];

    const enemy: SimEnemy = {
      id: Antibaryon.id,
      instanceId: "enemy_01",
      name: "Antibaryon",
      level: 90,
      hp: 10000,
      max_hp: 10000,
      resistance: 0.2,
      is_broken: false,
      vulnerability: 0,
      dmg_reduction: 0,
      weaken: 0,
      debuffCount: 0,
      activeDebuffs: {},
      activeBuffs: {},
      base_stats: { [ENEMY_SPD_ID]: 120, [ENEMY_ATK_ID]: 800, [ENEMY_HP_ID]: 10000 }
    };

    const report = runCombatSimulation(team, enemy, 3);
    
    console.log("=== HSR COMBAT SIMULATION LOGS ===");
    report.logs.forEach(log => console.log(log));
    console.log("==================================");
  });
});
