import { it, expect, describe } from "vitest";
import { runCombatSimulation } from "./hsr/simulator";
import { TeamMember, SimEnemy } from "./hsr/types";

const CHAR_HP_ID = '7383172e-f828-4298-a8cf-887d50ff4a28';
const CHAR_SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';
const ENEMY_HP_ID = 'dab1d58a-5e35-470a-a2d4-1bdddf3019a0';
const ENEMY_SPD_ID = 'b0bfd27b-0a5f-4329-a280-dc1c998446cb';
const ENEMY_ATK_ID = '7761c316-9c6b-4610-aa72-afcb80aeb1e9';

describe("HSR Mooncocoon Survival Logic", () => {
  it("should allow character to take one turn in Mooncocoon and fail at start of second turn", () => {
    const slowAlly: TeamMember = {
      characterId: 'slow-ally',
      name: "Slow Ally",
      element: "Physical",
      level: 80,
      eidolon: 0,
      hp: 100,
      max_hp: 100,
      shield: 0,
      abilityLevels: { basic: 1, skill: 1, ultimate: 1, talent: 1 },
      base_stats: { [CHAR_HP_ID]: 100, [CHAR_SPD_ID]: 100 }, 
      buffs: { atk_percent: 0, crit_rate: 0, crit_dmg: 0, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
      activeBuffs: {},
      activeDebuffs: {},
      lightcone: { base_stats: {}, scaling: 1 }
    };

    const tankAlly: TeamMember = {
      characterId: 'tank-ally',
      name: "Tank Ally",
      element: "Fire",
      level: 80,
      eidolon: 0,
      hp: 3000,
      max_hp: 3000,
      shield: 0,
      abilityLevels: { basic: 1, skill: 1, ultimate: 1, talent: 1 },
      base_stats: { [CHAR_HP_ID]: 3000, [CHAR_SPD_ID]: 100 }, 
      buffs: { atk_percent: 0, crit_rate: 0, crit_dmg: 0, dmg_boost: 0, def_ignore: 0, extra_multiplier: 0, extra_dmg: 0, res_pen: 0 },
      activeBuffs: {},
      activeDebuffs: {},
      lightcone: { base_stats: {}, scaling: 1 }
    };

    const fastEnemy: SimEnemy = {
      id: "50cf7b6b-c373-4ee8-ace8-13bf101e0f0f", // Antibaryon
      instanceId: "enemy-1",
      name: "Fast Enemy",
      level: 80,
      hp: 1000000,
      max_hp: 1000000,
      resistance: 0.2,
      elemental_res: {},
      is_broken: false,
      vulnerability: 0,
      dmg_reduction: 0,
      weaken: 0,
      debuffCount: 0,
      activeDebuffs: {},
      activeBuffs: {},
      base_stats: { [ENEMY_HP_ID]: 1000000, [ENEMY_SPD_ID]: 101, [ENEMY_ATK_ID]: 5000 }
    };

    const report = runCombatSimulation([slowAlly, tankAlly], fastEnemy, 10, { hasCastorice: true });

    console.log(report.logs.map(l => l.message).join("\n"));

    // 1. Check if Mooncocoon was triggered
    const triggered = report.logs.some(l => 
        l.message.includes("entered [Mooncocoon]") || 
        (l.subEntries && l.subEntries.some(s => s.includes("entered [Mooncocoon]")))
    );
    expect(triggered).toBe(true);

    // 2. Check if the character took a turn while in Mooncocoon
    const actingWhileCocooned = report.logs.some(l => 
        l.message.includes("is acting while in [Mooncocoon]") ||
        (l.subEntries && l.subEntries.some(s => s.includes("is acting while in [Mooncocoon]")))
    );
    expect(actingWhileCocooned).toBe(true);

    // 3. Check if they failed to recover eventually (either turn expiry or further damage)
    const collapsed = report.logs.some(l => 
        l.message.includes("collapsed") ||
        (l.subEntries && l.subEntries.some(s => s.includes("collapsed")))
    );
    expect(collapsed).toBe(true);
  });
});
