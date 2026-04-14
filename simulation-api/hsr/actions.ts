"use server";

import { createPublicClient } from "../supabase/server.js";
import { getEntityStats, getFullEntityById, getEntityAbilities } from "../supabase/queries.js";
import { calculateHsrDamage, type SimulationInput } from "./formulas.js";

// IDs from HSR_ID_MAPPING.md
const STAT_IDS = {
  CHAR_ATK: "c987f652-6a0b-487f-9e4b-af2c9b51c6aa",
  LC_ATK: "8e5af9db-3079-49ef-90c3-747b4ea00025",
  ENEMY_DEF: "7b58e059-a7ec-4535-a685-8961e5bc518d",
  ENEMY_RES_FIRE: "2c50d8d8-3d62-4e5d-8221-68f28d8cdddb", // Example for Fire
};

export async function runSkeletonSimulation(params: {
  charId: string;
  lcId: string;
  enemyId: string;
  charLevel: number;
  lcLevel: number;
  enemyLevel: number;
  superimposition: number;
}) {
  const [charStats, lcStats, enemyStats, abilities] = await Promise.all([
    getEntityStats(params.charId),
    getEntityStats(params.lcId),
    getEntityStats(params.enemyId),
    getEntityAbilities(params.charId)
  ]);

  // Helper to get stat value at level
  const getStatVal = (stats: any[], id: string, level: number) => {
    return stats.find(s => s.stat_id === id && s.level === level)?.value || 0;
  };

  // Build the simulation input
  const input: SimulationInput = {
    character: {
      level: params.charLevel,
      base_stats: { [STAT_IDS.CHAR_ATK]: getStatVal(charStats.data || [], STAT_IDS.CHAR_ATK, params.charLevel) },
      buffs: {
        atk_percent: 0, // Assume no buffs for base simulation
        crit_rate: 5,
        crit_dmg: 50,
        dmg_boost: 0,
        def_ignore: 0,
        extra_multiplier: 0,
        extra_dmg: 0,
        res_pen: 0
      }
    },
    lightcone: {
      base_stats: { [STAT_IDS.LC_ATK]: getStatVal(lcStats.data || [], STAT_IDS.LC_ATK, params.lcLevel) },
      scaling: 1.0 // This would be fetched from the LC ability superimposition table
    },
    enemy: {
      level: params.enemyLevel,
      resistance: 0.20, // Default 20%
      is_broken: false,
      vulnerability: 0,
      dmg_reduction: 0,
      weaken: 0
    },
    ability_multiplier: 1.0, // Default 100%
    scaling_stat_id: STAT_IDS.CHAR_ATK
  };

  return calculateHsrDamage(input);
}
