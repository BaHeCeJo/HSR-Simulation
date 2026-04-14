import { describe, it, expect } from 'vitest';
import { calculateHsrDamage, calculateSuperBreakDamage, SimulationInput } from './hsr/formulas';

describe('HSR Damage Formulas (Prydwen Verification)', () => {
  const SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';
  const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';

  it('calculates Tingyun "Benediction" Additional DMG correctly', () => {
    const input: SimulationInput = {
      character: {
        level: 80,
        base_stats: { 
          [ATK_ID]: 1200, 
          [SPD_ID]: 100 
        },
        buffs: {
          atk_percent: 150,
          crit_rate: 5,
          crit_dmg: 50,
          dmg_boost: 0,
          def_ignore: 0,
          extra_multiplier: 0,
          extra_dmg: 1200,
          res_pen: 0
        }
      },
      lightcone: {
        base_stats: {},
        scaling: 1.0
      },
      enemy: {
        level: 80,
        resistance: 0.20,
        is_broken: false,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0
      },
      ability_multiplier: 1.0,
      scaling_stat_id: ATK_ID
    };

    const result = calculateHsrDamage(input);
    expect(result.non_crit_dmg).toBe(1512);
  });

  it('calculates Super Break Damage correctly (Firefly/Boothill Model)', () => {
    const input: SimulationInput = {
      character: {
        level: 80,
        base_stats: { [SPD_ID]: 150 },
        buffs: {
          atk_percent: 0,
          crit_rate: 0,
          crit_dmg: 0,
          dmg_boost: 0,
          def_ignore: 0,
          extra_multiplier: 0,
          extra_dmg: 0,
          res_pen: 0
        }
      },
      lightcone: { base_stats: {}, scaling: 1.0 },
      enemy: {
        level: 80,
        resistance: 0.20,
        is_broken: true,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0,
        toughness_reduction: 30
      },
      ability_multiplier: 0,
      scaling_stat_id: ATK_ID
    };

    const breakEffect = 250;
    const result = calculateSuperBreakDamage(input, breakEffect, 30);
    expect(result).toBe(15823);
  });

  it('calculates Action Value correctly', () => {
    const input: SimulationInput = {
      character: {
        level: 80,
        base_stats: { [SPD_ID]: 134 },
        buffs: {
          atk_percent: 0,
          crit_rate: 0,
          crit_dmg: 0,
          dmg_boost: 0,
          def_ignore: 0,
          extra_multiplier: 0,
          extra_dmg: 0,
          res_pen: 0
        }
      },
      lightcone: { base_stats: {}, scaling: 1.0 },
      enemy: {
        level: 80,
        resistance: 0.20,
        is_broken: false,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0
      },
      ability_multiplier: 1.0,
      scaling_stat_id: ATK_ID
    };

    const result = calculateHsrDamage(input);
    expect(result.action_value).toBe(74.63);
  });

  it('calculates Solo Tingyun 1v1 Damage (Basic + Talent) correctly', () => {
    const ATK_VALUE = 2126;
    const input: SimulationInput = {
      character: {
        level: 80,
        base_stats: { 
          [ATK_ID]: 1000,
          [SPD_ID]: 112 
        },
        buffs: {
          atk_percent: 112.6,
          crit_rate: 5,
          crit_dmg: 50,
          dmg_boost: 0,
          def_ignore: 0,
          extra_multiplier: 0,
          extra_dmg: ATK_VALUE * 0.6,
          res_pen: 0
        }
      },
      lightcone: { base_stats: {}, scaling: 1.0 },
      enemy: {
        level: 80,
        resistance: 0.20,
        is_broken: false,
        vulnerability: 0,
        dmg_reduction: 0,
        weaken: 0
      },
      ability_multiplier: 1.0,
      scaling_stat_id: ATK_ID
    };

    const result = calculateHsrDamage(input);
    expect(result.non_crit_dmg).toBe(1224);
  });
});
