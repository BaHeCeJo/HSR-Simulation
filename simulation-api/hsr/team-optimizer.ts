import { runCombatSimulation, TeamMember, SimEnemy, SimReport } from "./simulator.js";
import { HSR_CHARACTER_KITS } from "./kits.js";

export interface OptimizationResult {
  bestTeam: TeamMember[];
  bestReport: SimReport;
  allResults: { teamNames: string[]; totalDamage: number }[];
  simulationsCount: number;
}

/**
 * Team Optimizer
 * Finds the highest damage 4-person team from a pool of available characters.
 */
export function optimizeTeam(
  availableMembers: TeamMember[],
  enemy: SimEnemy | SimEnemy[],
  maxCycles: number = 3,
  options: { hasCastorice?: boolean } = {}
): OptimizationResult {
  const results: { teamNames: string[]; totalDamage: number }[] = [];
  let bestReport: SimReport | null = null;
  let bestTeam: TeamMember[] = [];

  // 1. Generate combinations of 4 from the pool
  // Note: For large pools, this should be limited or use a heuristic.
  const combinations = getCombinations(availableMembers, 4);

  for (const team of combinations) {
    const report = runCombatSimulation(team, enemy, maxCycles, options);
    
    const teamNames = team.map(m => HSR_CHARACTER_KITS[m.characterId]?.name || "Unknown");
    results.push({ teamNames, totalDamage: report.totalDamage });

    if (!bestReport) {
      bestReport = report;
      bestTeam = team;
    } else {
      // Priority:
      // 1. Survived (isDefeated = false) > Defeated
      // 2. Faster clear (lower cyclesTaken)
      // 3. Higher damage (if cycles are the same)
      const isBetter = 
          (bestReport.isDefeated && !report.isDefeated) || 
          (bestReport.isDefeated === report.isDefeated && report.cyclesTaken < bestReport.cyclesTaken) ||
          (bestReport.isDefeated === report.isDefeated && report.cyclesTaken === bestReport.cyclesTaken && report.totalDamage > bestReport.totalDamage);

      if (isBetter) {
        bestReport = report;
        bestTeam = team;
      }
    }
  }

  // Sort results by damage
  results.sort((a, b) => b.totalDamage - a.totalDamage);

  return {
    bestTeam,
    bestReport: bestReport!,
    allResults: results,
    simulationsCount: combinations.length
  };
}

/**
 * Standard combinations algorithm (nCr)
 */
function getCombinations<T>(array: T[], n: number): T[][] {
  if (n === 0) return [[]];
  const result: T[][] = [];
  
  for (let i = 0; i <= array.length - n; i++) {
    const head = array.slice(i, i + 1);
    const tails = getCombinations(array.slice(i + 1), n - 1);
    for (const tail of tails) {
      result.push(head.concat(tail));
    }
  }
  return result;
}
