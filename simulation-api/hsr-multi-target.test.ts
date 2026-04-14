import { describe, it, expect } from 'vitest';
import { runCombatSimulation } from './hsr/simulator';
import { Acheron } from './hsr/characters/acheron';
import { Antibaryon } from './hsr/enemies/antibaryon';
import { Baryon } from './hsr/enemies/baryon';
import { TeamMember, SimEnemy } from './hsr/types';

describe('HSR Multi-Target Simulation', () => {
  it('should apply Blast and AoE damage correctly', () => {
    const team: TeamMember[] = [
      {
        characterId: Acheron.id,
        level: 80,
        hp: 3500,
        max_hp: 3500,
        eidolon: 0,
        abilityLevels: { basic: 6, skill: 10, ultimate: 10, talent: 10 },
        base_stats: {
          '3e4b082d-7943-440d-ae2c-8d31b0a370be': 101, // SPD
          'c987f652-6a0b-487f-9e4b-af2c9b51c6aa': 1500, // ATK
          '73868117-3df2-470d-945a-e389f9f04200': 1000, // DEF
        },
        buffs: {
          atk_percent: 0,
          crit_rate: 100, // Force crit for predictable damage
          crit_dmg: 100,
          dmg_boost: 0,
          def_ignore: 0,
          res_pen: 0,
          extra_multiplier: 0,
          extra_dmg: 0,
        },
        activeBuffs: {},
        activeDebuffs: {},
        lightcone: {
          base_stats: { 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa': 500 },
          scaling: 1.0,
        },
      }
    ];

    const enemies: SimEnemy[] = [
      {
        id: Antibaryon.id,
        instanceId: 'enemy_1',
        name: 'Antibaryon',
        level: 80,
        hp: 100000,
        max_hp: 100000,
        resistance: 0.2,
        elemental_res: {},
        is_broken: false,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0,
        debuffCount: 0,
        activeDebuffs: {},
        activeBuffs: {},
        base_stats: {
            'b0bfd27b-0a5f-4329-a280-dc1c998446cb': 80, // SPD (slow)
        },
      },
      {
        id: Baryon.id,
        instanceId: 'enemy_2',
        name: 'Baryon',
        level: 80,
        hp: 100000,
        max_hp: 100000,
        resistance: 0.2,
        elemental_res: {},
        is_broken: false,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0,
        debuffCount: 0,
        activeDebuffs: {},
        activeBuffs: {},
        base_stats: {
            'b0bfd27b-0a5f-4329-a280-dc1c998446cb': 80,
        },
      },
      {
        id: Baryon.id,
        instanceId: 'enemy_3',
        name: 'Baryon',
        level: 80,
        hp: 100000,
        max_hp: 100000,
        resistance: 0.2,
        elemental_res: {},
        is_broken: false,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0,
        debuffCount: 0,
        activeDebuffs: {},
        activeBuffs: {},
        base_stats: {
            'b0bfd27b-0a5f-4329-a280-dc1c998446cb': 80,
        },
      }
    ];

    const report = runCombatSimulation(team, enemies, 1);
    
    console.log("=== MULTI-TARGET COMBAT LOGS ===");
    report.logs.forEach(log => console.log(log));
    
    // Check if Acheron's Skill hit multiple enemies
    const skillLog = report.logs.find(l => l.type === 'action' && l.message.includes('Uses SKILL'));
    expect(skillLog).toBeDefined();
    
    // Should have 1 main hit + 1 adjacent hit (enemy_1 is first, so only enemy_2 is adjacent)
    // These hits are in subEntries
    const subEntries = skillLog?.subEntries || [];
    expect(subEntries.length).toBeGreaterThanOrEqual(2);
    expect(subEntries.some(l => l.includes('Hit main on Antibaryon (enemy_1)'))).toBe(true);
    expect(subEntries.some(l => l.includes('Hit adjacent on Baryon (enemy_2)'))).toBe(true);
  });
});
