export interface SimulationInput {
  character: {
    element: string;
    level: number;
    base_stats: Record<string, number>; // UUID -> Value
    buffs: {
      atk_percent: number;
      crit_rate: number;
      crit_dmg: number;
      dmg_boost: number;
      def_ignore: number;     // % of enemy DEF attacker ignores
      def_reduction: number;  // % reduction to enemy's DEF (e.g. Pela debuff)
      extra_multiplier: number; // Additive bonus to ability multiplier
      extra_dmg: number;      // Flat DMG addition
      res_pen: number;        // RES Penetration
      weaken: number;         // Attacker is weakened  Edeals X% less DMG
    };
  };
  lightcone: {
    base_stats: Record<string, number>;
    scaling: number;
  };
  enemy: {
    level: number;
    resistance: number;                    // Base RES fallback
    elemental_res?: Record<string, number>; // Per-element RES
    is_broken: boolean;
    vulnerability: number;    // Additive sum of all vulnerability sources (e.g. 0.10 = 10%)
    dmg_reduction: number;    // Pre-combined multiplicative DMG Mitigation (e.g. 0.20 = 20%)
    toughness_reduction?: number;
  };
  ability_multiplier: number; // e.g., 1.20 for 120%
  scaling_stat_id: string;
}

// ... (SimulationStep and SimulationResult remain same)


const LEVEL_MULTIPLIERS: Record<number, number> = {
  1: 54.0000, 2: 58.0000, 3: 62.0000, 4: 67.5264, 5: 70.5094,
  6: 73.5228, 7: 76.5660, 8: 79.6385, 9: 82.7395, 10: 85.8684,
  11: 91.4944, 12: 97.0680, 13: 102.5892, 14: 108.0579, 15: 113.4743,
  16: 118.8383, 17: 124.1499, 18: 129.4091, 19: 134.6159, 20: 139.7703,
  21: 149.3323, 22: 158.8011, 23: 168.1768, 24: 177.4594, 25: 186.6489,
  26: 195.7452, 27: 204.7484, 28: 213.6585, 29: 222.4754, 30: 231.1992,
  31: 246.4276, 32: 261.1810, 33: 275.4733, 34: 289.3179, 35: 302.7275,
  36: 315.7144, 37: 328.2905, 38: 340.4671, 39: 352.2554, 40: 363.6658,
  41: 408.1240, 42: 451.7883, 43: 494.6798, 44: 536.8188, 45: 578.2249,
  46: 618.9172, 47: 658.9138, 48: 698.2325, 49: 736.8905, 50: 774.9041,
  51: 871.0599, 52: 964.8705, 53: 1056.4206, 54: 1145.7910, 55: 1233.0585,
  56: 1318.2965, 57: 1401.5750, 58: 1482.9608, 59: 1562.5178, 60: 1640.3068,
  61: 1752.3215, 62: 1861.9011, 63: 1969.1242, 64: 2074.0659, 65: 2176.7983,
  66: 2277.3904, 67: 2375.9085, 68: 2472.4160, 69: 2566.9739, 70: 2659.6406,
  71: 2780.3044, 72: 2898.6022, 73: 3014.6029, 74: 3128.3729, 75: 3239.9758,
  76: 3349.4730, 77: 3456.9236, 78: 3562.3843, 79: 3665.9099, 80: 3767.5533,
  81: 3957.8618, 82: 4155.2118, 83: 4359.8638, 84: 4572.0878, 85: 4792.1641,
  86: 5020.3833, 87: 5257.0466, 88: 5502.4664, 89: 5756.9667, 90: 6020.8836,
  91: 6294.5654, 92: 6578.3734, 93: 6872.6823, 94: 7177.8806, 95: 7494.3713
};

// Coefficient used in the Break DMG formula: BaseDMG = coeff * LevelMult * MaxToughnessMult
const BREAK_BASE_DMG: Record<string, number> = {
  "Physical":  2.0,
  "Fire":      2.0,
  "Ice":       1.0,
  "Lightning": 1.0,
  "Wind":      1.5,
  "Quantum":   0.5,
  "Imaginary": 0.5,
};

/**
 * HSR Damage Formula
 *
 * DMG = BaseDMG * CRITMult * DMGBoostMult * WeakenMult
 *       * DEFMult * RESMult * VulnerabilityMult * DMGMitigationMult * BrokenMult
 *
 * - BaseDMG            = (AbilityMult + ExtraMult) * Stat + ExtraDMG
 * - CRITMult           = 1 + CRIT_DMG (on a crit hit); expected = 1 + CR*CD
 * - DMGBoostMult       = 1 + sum of all additive DMG boost sources
 * - WeakenMult         = 1 - Weaken  (attacker's debuff; does NOT apply to Break/Elation)
 * - DEFMult (enemy)    = (AttLv+20) / ((DefLv+20)*max(0, 1-%DEFIgnore-%DEFReduction) + (AttLv+20))
 * - RESMult            = clamp(1 - (RES - RES_PEN), 0.10, 2.00)
 * - VulnerabilityMult  = 1 + sum of all additive vulnerability sources
 * - DMGMitigationMult  = product of (1 - mitigationN)  Estored pre-combined in dmg_reduction
 * - BrokenMult         = 0.9 if enemy toughness intact, 1.0 if broken
 *
 * Note: Break DMG and Elation DMG are NOT affected by DMGBoostMult or WeakenMult.
 */
export function calculateHsrDamage(input: SimulationInput): SimulationResult {
  const { character, lightcone, enemy, ability_multiplier, scaling_stat_id } = input;
  const steps: SimulationStep[] = [];

  // 1. Scaling Stat = (CharBase + LCBase) * (1 + ATK%)
  const charBase = character.base_stats[scaling_stat_id] || 0;
  const lcBase = lightcone.base_stats[scaling_stat_id] || 0;
  const totalBase = charBase + lcBase;
  const totalStat = totalBase * (1 + (character.buffs.atk_percent / 100));

  steps.push({
    name: "Total Stat",
    formula: `(CharBase [${charBase}] + LCBase [${lcBase}]) * (1 + ATK% [${character.buffs.atk_percent}%])`,
    value: totalStat
  });

  // 2. BaseDMG = (AbilityMult + ExtraMult) * Stat + ExtraDMG
  const baseDmg = (ability_multiplier + (character.buffs.extra_multiplier / 100)) * totalStat + character.buffs.extra_dmg;

  steps.push({
    name: "Base DMG",
    formula: `(AbilityMult [${(ability_multiplier * 100).toFixed(1)}%] + ExtraMult [${character.buffs.extra_multiplier}%]) * Stat [${totalStat.toFixed(0)}] + ExtraDMG [${character.buffs.extra_dmg}]`,
    value: baseDmg
  });

  // 3. DMG Boost Multiplier = 1 + sum of all DMG boosts (additive)
  //    Does NOT apply to Break DMG or Elation DMG.
  const dmgBoostMult = 1 + (character.buffs.dmg_boost / 100);
  steps.push({
    name: "DMG Boost Multiplier",
    formula: `1 + DMG_Boost [${character.buffs.dmg_boost}%]`,
    value: dmgBoostMult
  });

  // 4. Weaken Multiplier = 1 - Weaken (attacker's outgoing DMG debuff)
  //    Does NOT apply to Break DMG or Elation DMG.
  const weakenMult = 1 - (character.buffs.weaken / 100);
  steps.push({
    name: "Weaken Multiplier",
    formula: `1 - Weaken [${character.buffs.weaken}%]`,
    value: weakenMult
  });

  // 5. DEF Multiplier (enemy)
  //    = (AttLv+20) / ((DefLv+20) * max(0, 1 - %DEFIgnore - %DEFReduction) + (AttLv+20))
  const defIgnore = (character.buffs.def_ignore || 0) / 100;
  const defReduction = (character.buffs.def_reduction || 0) / 100;
  const defFactor = Math.max(0, 1 - defIgnore - defReduction);
  const defMult = (character.level + 20) / ((enemy.level + 20) * defFactor + (character.level + 20));

  steps.push({
    name: "DEF Multiplier",
    formula: `(CharLv [${character.level}]+20) / ((EnemyLv [${enemy.level}]+20)*max(0,1-DefIgnore[${character.buffs.def_ignore}%]-DefReduction[${character.buffs.def_reduction}%]) + (CharLv+20))`,
    value: defMult
  });

  // 6. RES Multiplier = clamp(1 - (RES - RES_PEN), 0.10, 2.00)
  const baseRes = (enemy.elemental_res && enemy.elemental_res[character.element] !== undefined)
    ? enemy.elemental_res[character.element]
    : enemy.resistance;
  const rawResMult = 1 - (baseRes - (character.buffs.res_pen / 100));
  const resMult = Math.max(0.1, Math.min(2.0, rawResMult));

  steps.push({
    name: "RES Multiplier",
    formula: `clamp(1 - (RES_${character.element} [${(baseRes * 100).toFixed(1)}%] - RES_PEN [${character.buffs.res_pen}%]), 10%, 200%)`,
    value: resMult
  });

  // 7. Vulnerability Multiplier = 1 + sum of all vulnerability sources (additive)
  const vulnMult = 1 + (enemy.vulnerability / 100);
  steps.push({
    name: "Vulnerability Multiplier",
    formula: `1 + Vulnerability [${enemy.vulnerability}%]`,
    value: vulnMult
  });

  // 8. DMG Mitigation Multiplier  Esources stack multiplicatively; stored pre-combined
  const mitigationMult = 1 - (enemy.dmg_reduction / 100);
  steps.push({
    name: "DMG Mitigation Multiplier",
    formula: `1 - DMG_Reduction [${enemy.dmg_reduction}%]`,
    value: mitigationMult
  });

  // 9. Broken Multiplier = 0.9 if toughness intact, 1.0 if broken
  const brokenMult = enemy.is_broken ? 1.0 : 0.9;
  steps.push({
    name: "Broken Multiplier",
    formula: enemy.is_broken ? "Toughness Broken (ÁE.0)" : "Toughness Intact (ÁE.9)",
    value: brokenMult
  });

  // Final outgoing damage (non-crit)
  const outgoingDmg = baseDmg * dmgBoostMult * weakenMult * defMult * resMult * vulnMult * mitigationMult * brokenMult;

  steps.push({
    name: "Outgoing DMG (Non-Crit)",
    formula: "BaseDMG ÁEDMGBoost ÁEWeaken ÁEDEF ÁERES ÁEVulnerability ÁEMitigation ÁEBroken",
    value: outgoingDmg
  });

  // 10. CRIT Multiplier
  //     Actual hit: 1 + CRIT_DMG on crit, 1 otherwise.
  //     Expected value: 1 + CR ÁECD  (DoT and Break cannot CRIT).
  const critRate = Math.min(Math.max((character.buffs.crit_rate || 0) / 100, 0), 1);
  const critDmgMult = 1 + (character.buffs.crit_dmg / 100);
  const expectedCritMult = 1 + (critRate * (character.buffs.crit_dmg / 100));

  steps.push({
    name: "CRIT Multiplier (Expected)",
    formula: `1 + (CR [${(critRate * 100).toFixed(1)}%] ÁECD [${character.buffs.crit_dmg}%])`,
    value: expectedCritMult
  });

  const spd = character.base_stats['3e4b082d-7943-440d-ae2c-8d31b0a370be'] || 100;

  return {
    non_crit_dmg: Math.floor(outgoingDmg),
    crit_dmg: Math.floor(outgoingDmg * critDmgMult),
    expected_dmg: Math.floor(outgoingDmg * expectedCritMult),
    action_value: Number((10000 / spd).toFixed(2)),
    steps
  };
}

/**
 * Toughness Reduction Formula
 *
 * ToughnessReduction =
 *   (baseToughnessReduction + additiveToughnessReduction)
 *   ÁE(1 + %ToughnessReductionIncrease)
 *   ÁE(1 + clamp(%WeaknessBreakEfficiency, 0%, 300%) + %ToughnessVulnerability)
 *   ÁEabilityMultiplier
 *
 * @param base                     Base toughness reduction from the ability (e.g. 10, 20)
 * @param additive                 Flat additive bonus (default 0)
 * @param reductionIncrease        % increase to the base reduction itself (default 0)
 * @param breakEfficiency          % Weakness Break Efficiency  Ecapped at 300% (default 0)
 * @param toughnessVulnerability   % enemy vulnerability to toughness damage (default 0)
 * @param abilityMultiplier        Ability-specific multiplier (default 1.0)
 */
export function calculateToughnessReduction(
    base: number,
    additive: number = 0,
    reductionIncrease: number = 0,
    breakEfficiency: number = 0,
    toughnessVulnerability: number = 0,
    abilityMultiplier: number = 1.0
): number {
    const cappedEfficiency = Math.min(breakEfficiency, 300);
    return (base + additive)
        * (1 + reductionIncrease / 100)
        * (1 + cappedEfficiency / 100 + toughnessVulnerability / 100)
        * abilityMultiplier;
}

/**
 * True DMG
 *
 * True DMG is not an attack and bypasses all multipliers.
 * It is returned as-is (no CRIT, no DMG Boost, no DEF/RES/Vuln/Mitigation/Broken).
 */
export function calculateTrueDamage(amount: number): number {
    return Math.floor(amount);
}

// ─── Break Debuff Definitions ─────────────────────────────────────────────────

/** Element ↁEdebuff name */
export const BREAK_DEBUFF_BY_ELEMENT: Record<string, string> = {
    "Physical":  "Bleed",
    "Fire":      "Burn",
    "Ice":       "Freeze",
    "Lightning": "Shock",
    "Wind":      "Wind Shear",
    "Quantum":   "Entanglement",
    "Imaginary": "Imprisonment",
};

/** Duration (turns) of each break debuff */
export const BREAK_DEBUFF_DURATION: Record<string, number> = {
    "Bleed":        2,
    "Burn":         2,
    "Freeze":       1,
    "Shock":        2,
    "Wind Shear":   2,
    "Entanglement": 1,
    "Imprisonment": 1,
};

/** Initial Wind Shear / Entanglement stacks by enemy tier */
export const WIND_SHEAR_INITIAL_STACKS: Record<string, number> = {
    "normal": 1,
    "elite":  3,
    "boss":   3,
};
// Entanglement always starts at 1 stack; gains +1 per hit while active (max 5).
export const ENTANGLEMENT_INITIAL_STACKS = 1;
export const ENTANGLEMENT_MAX_STACKS = 5;
export const WIND_SHEAR_MAX_STACKS = 5;

// ─── Shared DEF/RES helpers ────────────────────────────────────────────────────

function breakDefMult(attackerLevel: number, enemyLevel: number, defIgnore: number, defReduction: number): number {
    const defFactor = Math.max(0, 1 - defIgnore - defReduction);
    return (attackerLevel + 20) / ((enemyLevel + 20) * defFactor + (attackerLevel + 20));
}

function breakResMult(baseRes: number, resPen: number): number {
    return Math.max(0.1, Math.min(2.0, 1 - (baseRes - resPen)));
}

// ─── Weakness Break DMG (on the triggering hit) ────────────────────────────────

/**
 * Instant Break DMG dealt at the moment toughness hits 0.
 *
 * BaseDMG = BREAK_BASE_DMG[element] ÁELevelMult ÁEMaxToughnessMult
 *
 * DMG = BaseDMG ÁE(1 + BreakEffect%) ÁEDEFMult ÁERESMult ÁEVulnMult ÁEMitigationMult ÁE0.9
 *
 * BrokenMult = 0.9  Ethe enemy was NOT yet in the Weakness Broken state when this is dealt.
 * Not affected by DMG Boost or Weaken.
 */
export function calculateBreakDamage(input: SimulationInput, breakEffect: number, maxToughness: number): number {
  const { character, enemy } = input;
  const levelMult = LEVEL_MULTIPLIERS[character.level] || 3767.5533;
  const maxToughnessMult = 0.5 + (maxToughness / 40);
  const baseDmg = (BREAK_BASE_DMG[character.element] || 1.0) * levelMult * maxToughnessMult;

  const defMult = breakDefMult(
      character.level, enemy.level,
      (character.buffs.def_ignore || 0) / 100,
      (character.buffs.def_reduction || 0) / 100
  );
  const baseRes = (enemy.elemental_res && enemy.elemental_res[character.element] !== undefined)
      ? enemy.elemental_res[character.element] : enemy.resistance;
  const resMult = breakResMult(baseRes, (character.buffs.res_pen || 0) / 100);
  const vulnMult  = 1 + (enemy.vulnerability / 100);
  const mitigMult = 1 - (enemy.dmg_reduction / 100);
  const brokenMult = 0.9; // enemy was unbroken when toughness hit 0

  return Math.floor(baseDmg * (1 + breakEffect / 100) * defMult * resMult * vulnMult * mitigMult * brokenMult);
}

// ─── Break DoT / Delayed DMG ──────────────────────────────────────────────────

export interface BreakDoTInput {
  debuffName:      string;   // 'Bleed' | 'Burn' | 'Shock' | 'Wind Shear' | 'Entanglement' | 'Freeze'
  attackerLevel:   number;
  breakEffect:     number;   // %
  breakDmgIncrease: number;  // % extra break DMG from equipment/eidolons (usually 0)
  defIgnore:       number;   // fraction
  defReduction:    number;   // fraction
  resPen:          number;   // fraction
  // Enemy state at the time of the tick
  enemyLevel:      number;
  baseRes:         number;
  vulnerability:   number;   // %
  dmgReduction:    number;   // %
  isBroken:        boolean;
  // Debuff-specific values
  stacks?:         number;   // Wind Shear, Entanglement
  maxToughness?:   number;   // Entanglement
  maxHp?:          number;   // Bleed
  isNormalEnemy?:  boolean;  // Bleed coefficient (normal vs elite/boss)
}

/**
 * Break DoT / Delayed DMG formula:
 *
 * DMG = BaseDMG ÁEAbilityMult(1) ÁE(1 + BreakEffect%) ÁE(1 + BreakDMGIncrease%)
 *       ÁEDEFMult ÁERESMult ÁEVulnMult ÁEMitigationMult ÁEBrokenMult
 *
 * Not affected by DMG Boost or Weaken.
 */
export function calculateBreakDoTDamage(inp: BreakDoTInput): number {
    const levelMult = LEVEL_MULTIPLIERS[inp.attackerLevel] || 3767.5533;
    const maxToughMult = inp.maxToughness ? 0.5 + (inp.maxToughness / 40) : 1.0;
    const stacks = inp.stacks ?? 1;

    let baseDmg: number;
    switch (inp.debuffName) {
        case "Bleed": {
            const bleedCoeff = inp.isNormalEnemy ? 0.16 : 0.07;
            const bleedRaw = bleedCoeff * (inp.maxHp || 0);
            const bleedCap = 2 * levelMult * maxToughMult;
            baseDmg = Math.min(bleedRaw, bleedCap);
            break;
        }
        case "Burn":         baseDmg = 1.0 * levelMult; break;
        case "Freeze":       baseDmg = 1.0 * levelMult; break; // dealt once on unfreeze
        case "Shock":        baseDmg = 2.0 * levelMult; break;
        case "Wind Shear":   baseDmg = 1.0 * stacks * levelMult; break;
        case "Entanglement": baseDmg = 0.6 * stacks * levelMult * maxToughMult; break;
        default:             baseDmg = 0;
    }

    const defMult  = breakDefMult(inp.attackerLevel, inp.enemyLevel, inp.defIgnore, inp.defReduction);
    const resMult  = breakResMult(inp.baseRes, inp.resPen);
    const vulnMult = 1 + (inp.vulnerability / 100);
    const mitigMult = 1 - (inp.dmgReduction / 100);
    const brokenMult = inp.isBroken ? 1.0 : 0.9;

    return Math.floor(
        baseDmg
        * (1 + inp.breakEffect / 100)
        * (1 + inp.breakDmgIncrease / 100)
        * defMult * resMult * vulnMult * mitigMult * brokenMult
    );
}

// ─── Elation DMG ──────────────────────────────────────────────────────────────

/**
 * Elation-specific Level Multipliers (separate table from the standard one).
 * These values are approximately 2ÁEthe standard LEVEL_MULTIPLIERS.
 * Only levels 1 E0 are defined from game data; levels 81 E5 fall back to 2ÁEstandard.
 */
const ELATION_LEVEL_MULTIPLIERS: Record<number, number> = {
  1: 108.00000, 2: 116.00000, 3: 124.00000, 4: 135.05276, 5: 141.01880,
  6: 147.04564, 7: 153.13210, 8: 159.27693, 9: 165.47893, 10: 171.73688,
  11: 182.98882, 12: 194.13596, 13: 205.17833, 14: 216.11589, 15: 226.94867,
  16: 237.67665, 17: 248.29984, 18: 258.81824, 19: 269.23184, 20: 279.54068,
  21: 298.66458, 22: 317.60223, 23: 336.35364, 24: 354.91880, 25: 373.29770,
  26: 391.49036, 27: 409.49677, 28: 427.31693, 29: 444.95080, 30: 462.39847,
  31: 492.85513, 32: 522.36194, 33: 550.94666, 34: 578.63580, 35: 605.45496,
  36: 631.42880, 37: 656.58093, 38: 680.93427, 39: 704.51074, 40: 727.33160,
  41: 816.24800, 42: 903.57660, 43: 989.35956, 44: 1073.6376, 45: 1156.4498,
  46: 1237.8344, 47: 1317.8276, 48: 1396.4651, 49: 1473.7810, 50: 1549.8082,
  51: 1742.1199, 52: 1929.7411, 53: 2112.8413, 54: 2291.5820, 55: 2466.1170,
  56: 2636.5930, 57: 2803.1501, 58: 2965.9216, 59: 3125.0356, 60: 3280.6135,
  61: 3504.6430, 62: 3723.8022, 63: 3938.2483, 64: 4148.1320, 65: 4353.5967,
  66: 4554.7810, 67: 4751.8170, 68: 4944.8320, 69: 5133.9478, 70: 5319.2812,
  71: 5560.6090, 72: 5797.2046, 73: 6029.2060, 74: 6256.7460, 75: 6479.9517,
  76: 6698.9463, 77: 6913.8470, 78: 7124.7686, 79: 7331.8200, 80: 7535.1070,
};

/** Multiplier per hit for Aha's "Let There Be Laughter" (no Elation chars present). */
export const AHA_LTBL_HIT_MULTIPLIER = 0.5;

export interface ElationDmgInput {
  /** The Elation character's (or Aha's) level, used to look up ELATION_LEVEL_MULTIPLIERS. */
  attackerLevel: number;
  /** The attacker's element (used for RES look-up on the target). */
  element: string;
  // ── Elation-specific stats ──────────────────────────────────────────────────
  /** "Elation" advanced stat value in % (e.g. 50.0 = 50%). Increases Elation DMG. */
  elation: number;
  /** "Merrymake" character stat in % (e.g. 20.0 = 20%). Increases Elation DMG. */
  merrymake: number;
  /**
   * Points used for the Punchline Multiplier formula.
   * Pass CertifiedBanger combined points if the char has an active CB state,
   * otherwise pass the current team Punchline count.
   */
  punchlinePoints: number;
  // ── DEF multiplier inputs ───────────────────────────────────────────────────
  /** DEF Ignore fraction (e.g. 0.20 for 20%). */
  defIgnore: number;
  /** DEF Reduction fraction (e.g. 0.10 for 10%). */
  defReduction: number;
  /** RES Penetration fraction (e.g. 0.10 for 10%). */
  resPen: number;
  // ── CRIT ───────────────────────────────────────────────────────────────────
  /** CRIT Rate in % (e.g. 65.0). Expected-value CRIT is used. */
  crit_rate: number;
  /** CRIT DMG in % (e.g. 150.0). */
  crit_dmg: number;
  // ── Ability scaling ─────────────────────────────────────────────────────────
  /** The ability multiplier from the kit (e.g. 0.8 for 80%). */
  abilityMultiplier: number;
  // ── Enemy state ─────────────────────────────────────────────────────────────
  enemyLevel: number;
  elemental_res: Record<string, number>;
  baseRes: number;
  is_broken: boolean;
  vulnerability: number;  // %
  dmg_reduction: number;  // %
}

/**
 * Elation DMG Formula:
 *
 * ElationDMG = BaseDMG ÁECRITMult ÁEElationMult ÁEPunchlineMult ÁEMerrymakeMult
 *              ÁEDEFMult ÁERESMult ÁEVulnMult ÁEMitigMult ÁEBrokenMult
 *
 * BaseDMG = AbilityMultiplier ÁEElationLevelMultiplier   (NOT stat-based)
 * ElationMult   = 1 + Elation%
 * PunchlineMult = 1 + (P ÁE5) / (P + 240)   where P = CertifiedBangerPoints (if active) else Punchline
 * MerrymakeMult = 1 + Merrymake%
 *
 * NOT affected by DMG Boost or Weaken.
 */
export function calculateElationDamage(inp: ElationDmgInput): number {
  // BaseDMG uses Elation-specific level table; fall back to 2ÁEstandard if level > 80
  const levelMult = ELATION_LEVEL_MULTIPLIERS[inp.attackerLevel]
    ?? ((LEVEL_MULTIPLIERS[inp.attackerLevel] ?? 7535.1070) * 2);

  const baseDmg = inp.abilityMultiplier * levelMult;

  const elationMult   = 1 + (inp.elation   / 100);
  const merrymakeMult = 1 + (inp.merrymake / 100);

  const p = inp.punchlinePoints;
  const punchlineMult = 1 + (p * 5) / (p + 240); // = 1 when p = 0

  const defMult = breakDefMult(inp.attackerLevel, inp.enemyLevel, inp.defIgnore, inp.defReduction);

  const baseRes = (inp.elemental_res?.[inp.element] !== undefined)
    ? inp.elemental_res[inp.element]
    : inp.baseRes;
  const resMult = breakResMult(baseRes, inp.resPen);

  const vulnMult   = 1 + (inp.vulnerability / 100);
  const mitigMult  = 1 - (inp.dmg_reduction / 100);
  const brokenMult = inp.is_broken ? 1.0 : 0.9;

  // Expected-value CRIT (DoT and Break cannot crit, but Elation DMG can)
  const critRate = Math.min(Math.max(inp.crit_rate / 100, 0), 1);
  const critMult = 1 + critRate * (inp.crit_dmg / 100);

  return Math.floor(
    baseDmg * critMult
    * elationMult * punchlineMult * merrymakeMult
    * defMult * resMult * vulnMult * mitigMult * brokenMult
  );
}

// ─── Healing ──────────────────────────────────────────────────────────────────

export interface HealInput {
  /** The scaling stat value (e.g. ATK, HP, or DEF of the healer). */
  stat: number;
  /** % of the stat used for healing (e.g. 40.0 for 40%). */
  percentage: number;
  /** Flat additive healing bonus. */
  additive: number;
  /** Outgoing Healing Boost % from the healer (e.g. 20.0 for 20%). */
  outgoing_healing_boost: number;
  /** Incoming Healing Boost % from the target (passive buff on the target). */
  incoming_healing_boost: number;
  /** Incoming Healing Reduction % from the target (debuff on the target, reduces healing received). */
  incoming_healing_reduction: number;
}

/**
 * HealingAmount = (Stat ÁEPercentage% + AdditiveHealing) ÁEHealingBoostMultiplier
 *
 * HealingBoostMultiplier = 1 + OutgoingHealingBoostHealer
 *                            + IncomingHealingBoostTarget
 *                            ∁EIncomingHealingReductionTarget
 *
 * The multiplier is clamped to a minimum of 0 (cannot produce negative healing).
 */
export function calculateHeal(inp: HealInput): number {
  const base = inp.stat * (inp.percentage / 100) + inp.additive;
  const mult = 1
    + (inp.outgoing_healing_boost  / 100)
    + (inp.incoming_healing_boost  / 100)
    - (inp.incoming_healing_reduction / 100);
  return Math.floor(base * Math.max(0, mult));
}

// ─── Shields ──────────────────────────────────────────────────────────────────

export interface ShieldInput {
  /** The scaling stat value (e.g. DEF, HP of the shield-caster). */
  stat: number;
  /** % of the stat used for the shield value (e.g. 30.0 for 30%). */
  percentage: number;
  /** Flat additive shield bonus. */
  additive: number;
  /** Shield Bonus % from relics, passives, etc. (e.g. 20.0 for 20%). */
  shield_bonus: number;
}

/**
 * ShieldValue = (Stat ÁEStatScaling% + AdditiveShieldBonus) ÁE(1 + ShieldBonus%)
 */
export function calculateShield(inp: ShieldInput): number {
  const base = inp.stat * (inp.percentage / 100) + inp.additive;
  return Math.floor(base * (1 + inp.shield_bonus / 100));
}

// ─── Super Break ──────────────────────────────────────────────────────────────

/**
 * Super Break DMG:
 * (ToughnessReduction / 10) ÁELevelCoeff ÁE(1 + BreakEffect%) ÁEDEFMult ÁERESMult ÁEVulnMult ÁEMitigMult
 * Always on a broken enemy ↁEBrokenMult = 1.0 (not included in formula above).
 */
export function calculateSuperBreakDamage(input: SimulationInput, breakEffect: number, toughnessReduction: number): number {
  const { character, enemy } = input;
  const levelCoeff = LEVEL_MULTIPLIERS[character.level] || 3767.5533;

  const defMult = breakDefMult(
      character.level, enemy.level,
      (character.buffs.def_ignore || 0) / 100,
      (character.buffs.def_reduction || 0) / 100
  );
  const baseRes = (enemy.elemental_res && enemy.elemental_res[character.element] !== undefined)
      ? enemy.elemental_res[character.element] : enemy.resistance;
  const resMult  = breakResMult(baseRes, (character.buffs.res_pen || 0) / 100);
  const vulnMult = 1 + (enemy.vulnerability / 100);
  const mitigMult = 1 - (enemy.dmg_reduction / 100);

  return Math.floor((toughnessReduction / 10) * levelCoeff * (1 + breakEffect / 100) * defMult * resMult * vulnMult * mitigMult);
}
