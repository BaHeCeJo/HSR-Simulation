import {
    calculateHsrDamage, calculateBreakDamage, calculateSuperBreakDamage,
    calculateToughnessReduction, calculateTrueDamage,
    calculateBreakDoTDamage,
    calculateElationDamage,
    BREAK_DEBUFF_BY_ELEMENT, BREAK_DEBUFF_DURATION,
    WIND_SHEAR_INITIAL_STACKS, WIND_SHEAR_MAX_STACKS,
    ENTANGLEMENT_INITIAL_STACKS, ENTANGLEMENT_MAX_STACKS,
    AHA_LTBL_HIT_MULTIPLIER,
} from "./formulas.js";
import type { SimulationInput, SimulationResult } from "./formulas.js";
import { HSR_CHARACTER_KITS, HSR_ENEMY_KITS } from "./registry.js";
import type { SimState, TeamMember, SimEnemy, SimReport, Action, Wave, LogEntry, StatusEffect, SummonUnit, PendingFollowUp, PendingExtraTurn, CombatTarget, ShieldInstance, ZoneEffect, TerritoryEffect } from "./types.js";
import type { CC_DEBUFFS } from "./types.js";

// Definitive UUIDs from HSR_ID_MAPPING.md
const CHAR_HP_ID = '7383172e-f828-4298-a8cf-887d50ff4a28';
const CHAR_SPD_ID = '3e4b082d-7943-440d-ae2c-8d31b0a370be';
const CHAR_BE_ID = '268dd0de-bada-4dd1-ae1a-e1019290dab7';
const ENEMY_HP_ID = 'dab1d58a-5e35-470a-a2d4-1bdddf3019a0';
const ENEMY_SPD_ID = 'b0bfd27b-0a5f-4329-a280-dc1c998446cb';
const ENEMY_TOUGHNESS_ID = '50ff424d-9428-46e2-8f3e-8968dacbb6bd';

const ATK_ID = 'c987f652-6a0b-487f-9e4b-af2c9b51c6aa';
const DEF_ID = '73868117-3df2-470d-945a-e389f9f04200';
const CRIT_RATE_ID = 'a62e3a38-743a-41f8-8523-aec4ef998c84';
const CRIT_DMG_ID = 'a93e523a-7852-4580-b2ef-03467e214bcd';

// ── Elation path stat UUIDs ──────────────────────────────────────────────────
// TODO: Replace these placeholders with the real UUIDs from HSR_ID_MAPPING.md
// once the Elation and Merrymake advanced stats are mapped.
const ELATION_STAT_ID  = 'PLACEHOLDER_ELATION_UUID';  // "Elation" advanced stat
const MERRYMAKE_STAT_ID = 'PLACEHOLDER_MERRYMAKE_UUID'; // "Merrymake" character stat

const ELEMENT_MAP: Record<string, string> = {
  "441500bc-47dc-452f-9f1f-f0aaa142ce62": "Physical",
  "2c50d8d8-3d62-4e5d-8221-68f28d8cdddb": "Fire",
  "cc934b70-7aca-46dc-beb2-0aafc221e2ec": "Ice",
  "3de09fc5-7cb1-412f-aac0-0ebb0ba905e8": "Lightning",
  "4c775af5-281e-4bbb-8ca2-b2e3f20f3c18": "Wind",
  "9deee2d8-f7bf-41b7-829e-2485837784df": "Quantum",
  "176151ff-8d54-4b1b-98fb-03ef410d7371": "Imaginary"
};

const ACHERON_ID = "f06222e4-d23d-4ac2-86ff-3a6cc389b812";

// Base aggro values by path (higher = more likely to be targeted)
const BASE_AGGRO_BY_PATH: Record<string, number> = {
  "The Hunt":   3,
  "Erudition":  3,
  "Harmony":    4,
  "Nihility":   4,
  "Abundance":  4,
  "Remembrance": 4,
  "Elation":    4,
  "Destruction": 5,
  "Preservation": 6,
};

function getBaseAggro(path: string | undefined): number {
  return BASE_AGGRO_BY_PATH[path ?? ""] ?? 4;
}

function computeAggro(member: TeamMember): number {
  const base = getBaseAggro(member.path);
  return base * (1 + (member.aggroModifier ?? 0));
}

/**
 * Computes Aha's SPD based on the Elation characters' speeds.
 * SPDAha = 80 + SPD_1st/5 + SPD_2nd/10 + SPD_3rd/20 + SPD_4th/40
 */
function computeAhaSpd(team: TeamMember[]): number {
  const divisors = [5, 10, 20, 40];
  const elationSpeeds = team
    .filter(m => {
      const kit = HSR_CHARACTER_KITS[m.characterId];
      return (m.path || kit?.path) === "Elation";
    })
    .map(m => m.base_stats[CHAR_SPD_ID] || 100)
    .sort((a, b) => b - a); // descending  Efastest first

  let spd = 80;
  for (let i = 0; i < elationSpeeds.length && i < 4; i++) {
    spd += elationSpeeds[i] / divisors[i];
  }
  return spd;
}

/**
 * Aggro-weighted selection from allies + targetable Memosprites.
 * Returns a CombatTarget (either a TeamMember ally or a SummonUnit Memosprite).
 * Use this for enemy targeting; it gives Memosprites their own aggro weight.
 */
function selectCombatTargetFn(
  team: TeamMember[],
  summons: SummonUnit[],
  isBounce: boolean
): CombatTarget | null {
  const aliveAllies: CombatTarget[] = team
    .filter(m => !m.isDowned && m.hp > 0)
    .map(m => ({ kind: 'ally' as const, member: m }));

  // Memosprites that can be targeted by enemies
  const aliveMemos: CombatTarget[] = summons
    .filter(s => s.kind === 'memosprite' && s.canBeTargetedByEnemies && !s.isDowned && s.hp > 0)
    .map(s => ({ kind: 'memo' as const, summon: s }));

  const pool = [...aliveAllies, ...aliveMemos];
  if (pool.length === 0) return null;

  if (isBounce) {
    return pool[Math.floor(Math.random() * pool.length)] ?? null;
  }

  const weights = pool.map(t =>
    t.kind === 'ally'
      ? computeAggro(t.member)
      : t.summon.aggroValue
  );
  const total = weights.reduce((s, w) => s + w, 0);
  let roll = Math.random() * total;
  for (let i = 0; i < pool.length; i++) {
    roll -= weights[i] ?? 0;
    if (roll <= 0) return pool[i] ?? null;
  }
  return pool[pool.length - 1] ?? null;
}

/**
 * Weighted random selection using aggro weights.
 * For bounce attacks, every alive member has equal probability.
 */
function selectAggroTarget(team: TeamMember[], isBounce: boolean): TeamMember | null {
  const alive = team.filter(m => !m.isDowned && m.hp > 0);
  if (alive.length === 0) return null;
  if (isBounce) {
    return alive[Math.floor(Math.random() * alive.length)];
  }
  const weights = alive.map(m => computeAggro(m));
  const total = weights.reduce((s, w) => s + w, 0);
  let roll = Math.random() * total;
  for (let i = 0; i < alive.length; i++) {
    roll -= weights[i];
    if (roll <= 0) return alive[i];
  }
  return alive[alive.length - 1];
}

const ALLY_COLORS = ['#60a5fa', '#34d399', '#fbbf24', '#f87171', '#a78bfa'];
const ENEMY_COLORS = ['#fb7185', '#f472b6', '#fb923c', '#e879f9', '#94a3b8'];

let actorColorMap: Record<string, string> = {};

function getActorColor(id: string, instanceId: string, type: 'ally' | 'enemy'): string {
    const key = instanceId || id;
    if (actorColorMap[key]) return actorColorMap[key];
    
    const palette = type === 'ally' ? ALLY_COLORS : ENEMY_COLORS;
    const index = Object.keys(actorColorMap).length % palette.length;
    actorColorMap[key] = palette[index];
    return actorColorMap[key];
}

function addLog(state: SimState, entry: Omit<LogEntry, 'av'>) {
    const fullEntry: LogEntry = {
        ...entry,
        av: Number(state.currentAV.toFixed(1))
    };

    if (entry.type === 'event' && state.logs.length > 0) {
        const lastLog = state.logs[state.logs.length - 1];
        const isSameActor = !entry.actor || (
            (lastLog.actor && 
             entry.actor.id === lastLog.actor.id && 
             entry.actor.instanceId === lastLog.actor.instanceId)
        );

        if (isSameActor && (lastLog.type === 'action' || lastLog.type === 'turn')) {
            if (!lastLog.subEntries) lastLog.subEntries = [];
            lastLog.subEntries.push(entry.message);
            return;
        }
    }

    state.logs.push(fullEntry);
}

function resolveDatabaseScaling(
  abilities: any[],
  slotName: string,
  level: number,
  scalingConfig: { attribute_id?: string; attribute_index?: number; default_multiplier: number }
): number {
  if (!abilities || abilities.length === 0) return scalingConfig.default_multiplier;

  const ability = abilities.find(ab => {
    const name = ab.section_ability_definitions?.name?.en || "";
    return name.toLowerCase().includes(slotName.toLowerCase());
  });

  if (!ability || !ability.entity_ability_scaling) return scalingConfig.default_multiplier;

  const actualLevel = slotName.toLowerCase().includes("basic") ? Math.min(level, 6) : Math.min(level, 10);

  let scaling;
  if (scalingConfig.attribute_id) {
    scaling = ability.entity_ability_scaling.find((s: any) => 
        s.level === actualLevel && 
        s.section_ability_attributes?.id === scalingConfig.attribute_id
    );
  }

  if (!scaling) {
    const index = scalingConfig.attribute_index ?? 0;
    scaling = ability.entity_ability_scaling.find((s: any) => s.level === actualLevel && s.attribute_index === index);
  }
  
  if (scaling) {
    return scaling.value_type === 'percent' ? scaling.value / 100 : scaling.value;
  }

  return scalingConfig.default_multiplier;
}

function getPrimaryScalingConfig(raw: any): any {
  if (!raw) return null;
  if (raw.default_multiplier) return raw;
  return raw.stygian_resurge || raw.main || Object.values(raw)[0];
}

function initializeEnemy(e: SimEnemy): SimEnemy {
    const hp = e.hp || e.base_stats[ENEMY_HP_ID] || 10000;
    const base_res = e.resistance ?? 0.2;
    const toughness = e.toughness || e.base_stats[ENEMY_TOUGHNESS_ID] || 30; // Default 30 (1 bar)
    
    const elemental_res: Record<string, number> = (e.elemental_res && Object.keys(e.elemental_res).length > 0)
      ? { ...e.elemental_res }
      : {};
    
    Object.entries(ELEMENT_MAP).forEach(([statId, elementName]) => {
        if (elemental_res[elementName] !== undefined) return;
        if (e.base_stats[statId] !== undefined) {
            elemental_res[elementName] = e.base_stats[statId] / 100;
        } else {
            elemental_res[elementName] = base_res;
        }
    });

    const weaknesses = e.weaknesses || Object.entries(elemental_res)
        .filter(([el, res]) => res < 0.2)
        .map(([el, res]) => el);

    return {
        ...e,
        hp,
        max_hp: e.max_hp || hp,
        toughness,
        max_toughness: e.max_toughness || toughness,
        extra_toughness_bars: e.extra_toughness_bars ? [...e.extra_toughness_bars] : [],
        max_extra_toughness_bars: e.max_extra_toughness_bars ? [...e.max_extra_toughness_bars] : [],
        tier: e.tier || 'normal',
        weaknesses,
        resistance: base_res,
        elemental_res,
        activeDebuffs: { ...e.activeDebuffs },
        activeBuffs: { ...e.activeBuffs }
    };
}

function formatStatus(obj: Record<string, StatusEffect>, isDebuff: boolean = false): string {
    const entries = Object.entries(obj);
    if (entries.length === 0) return "None";
    return entries.map(([key, effect]) => {
        let effectStrings: string[] = [];
        if (effect.value !== undefined) {
            const prefix = isDebuff ? '-' : '+';
            effectStrings.push(`${prefix}${effect.value}% ${effect.stat || ''}`.trim());
        }
        if (effect.effects) {
            effect.effects.forEach(e => {
                const prefix = isDebuff ? '-' : '+';
                effectStrings.push(`${prefix}${e.value}% ${e.stat}`.trim());
            });
        }
        const effectsPart = effectStrings.length > 0 ? `, ${effectStrings.join(", ")}` : "";
        return `${key} (${effect.duration} turns${effectsPart})`;
    }).join("; ");
}


function setupWave(state: SimState, waveIndex: number) {
  const wave = state.waves[waveIndex];
  state.currentWaveIndex = waveIndex;
  state.enemyPool = [...wave.enemyPool].map(initializeEnemy);
  state.enemies = [null, null, null, null, null];
  
  addLog(state, { type: 'wave', message: `WAVE ${waveIndex + 1} START` });

  wave.initialEnemies.forEach((e, i) => {
    if (e) {
      const initialized = initializeEnemy(e);
      state.enemies[i] = initialized;
      const spd = initialized.base_stats[ENEMY_SPD_ID] || 100;
      
      const enemyKit = HSR_ENEMY_KITS[initialized.id];
      if (enemyKit?.hooks?.onBattleStart) enemyKit.hooks.onBattleStart(state, initialized);

      state.avQueue.push({ 
        id: initialized.id, 
        instanceId: initialized.instanceId, 
        nextAV: state.currentAV + 10000 / spd 
      });

      const resList = Object.entries(initialized.elemental_res)
        .map(([el, val]) => `${el}: ${(val * 100).toFixed(0)}%`)
        .join(", ");
      
      addLog(state, { 
          type: 'info', 
          message: `Slot ${i+1}: ${initialized.name} (HP=${initialized.hp.toFixed(0)}, SPD=${spd}, RES=[${resList}], TGH=${initialized.toughness.toFixed(0)}/${initialized.max_toughness.toFixed(0)}, WEAK=[${initialized.weaknesses.join(", ")}])`,
          actor: { 
              id: initialized.id, 
              instanceId: initialized.instanceId, 
              name: initialized.name,
              type: 'enemy',
              color: getActorColor(initialized.id, initialized.instanceId, 'enemy')
          }
      });
    }
  });
}

function checkEnemies(state: SimState) {
    for (let i = 0; i < state.enemies.length; i++) {
        const e = state.enemies[i];
        if (e && e.hp <= 0) {
            addLog(state, { 
                type: 'event', 
                message: `${e.name} defeated!`,
                actor: { 
                    id: e.id, 
                    instanceId: e.instanceId, 
                    name: e.name,
                    type: 'enemy',
                    color: getActorColor(e.id, e.instanceId, 'enemy')
                }
            });

            state.team.forEach(m => {
                const k = HSR_CHARACTER_KITS[m.characterId];
                if (k?.hooks?.onEnemyDefeated) {
                    k.hooks.onEnemyDefeated(state, m, e);
                }
            });

            state.enemies[i] = null;
            state.avQueue = state.avQueue.filter(a => a.instanceId !== e.instanceId || a.id !== e.id);
            
            if (state.enemyPool.length > 0) {
                const poolIdx = Math.floor(Math.random() * state.enemyPool.length);
                const nextEnemy = state.enemyPool.splice(poolIdx, 1)[0];
                state.enemies[i] = nextEnemy;
                const spd = nextEnemy.base_stats[ENEMY_SPD_ID] || 100;
                
                state.avQueue.push({ 
                  id: nextEnemy.id, 
                  instanceId: nextEnemy.instanceId, 
                  nextAV: state.currentAV + 10000 / spd 
                });
                
                addLog(state, { 
                    type: 'event', 
                    message: `${nextEnemy.name} enters the field in Slot ${i+1}!`,
                    actor: {
                        id: nextEnemy.id,
                        instanceId: nextEnemy.instanceId,
                        name: nextEnemy.name,
                        type: 'enemy',
                        color: getActorColor(nextEnemy.id, nextEnemy.instanceId, 'enemy')
                    }
                });
                
                const enemyKit = HSR_ENEMY_KITS[nextEnemy.id];
                if (enemyKit?.hooks?.onBattleStart) enemyKit.hooks.onBattleStart(state, nextEnemy);
            }
        }
    }
}

export function runCombatSimulation(
  originalTeam: TeamMember[],
  originalEnemies: SimEnemy | SimEnemy[] | Wave[],
  maxCycles: number = 10,
  options: { hasCastorice?: boolean; skipLogs?: boolean } = {}
): SimReport {
  actorColorMap = {};
  const maxAV = 150 + (maxCycles - 1) * 100;

  const team = originalTeam.map(m => {
    const kit = HSR_CHARACTER_KITS[m.characterId];
    const hp = m.hp || m.base_stats[CHAR_HP_ID] || 3000;
    const toughness = m.toughness || 100;
    return {
      ...m,
      name: m.name || kit?.name || m.characterId,
      element: m.element || kit?.element || "Physical",
      hp,
      max_hp: m.max_hp || hp,
      shield: m.shield || 0,
      activeShields: m.activeShields ? [...m.activeShields] : [],
      toughness,
      max_toughness: m.max_toughness || toughness,
      is_broken: m.is_broken || false,
      aggroModifier: m.aggroModifier ?? 0,
      buffs: {
        ...m.buffs,
        outgoing_healing_boost:    m.buffs.outgoing_healing_boost    ?? 0,
        incoming_healing_boost:    m.buffs.incoming_healing_boost    ?? 0,
        incoming_healing_reduction: m.buffs.incoming_healing_reduction ?? 0,
        shield_bonus:              m.buffs.shield_bonus               ?? 0,
      },
      activeBuffs: { ...m.activeBuffs },
      activeDebuffs: { ...m.activeDebuffs }
    };
  });

  let waves: Wave[] = [];
  if (Array.isArray(originalEnemies) && originalEnemies.length > 0 && 'initialEnemies' in (originalEnemies[0] as any)) {
      waves = originalEnemies as Wave[];
  } else {
      const enemiesArray = Array.isArray(originalEnemies) ? (originalEnemies as SimEnemy[]) : [originalEnemies as SimEnemy];
      const initialEnemies: (SimEnemy | null)[] = [null, null, null, null, null];
      const enemyPool: SimEnemy[] = [];
      enemiesArray.forEach((e, i) => { 
          if (i < 5) initialEnemies[i] = e; 
          else enemyPool.push(e);
      });
      waves = [{ initialEnemies, enemyPool }];
  }

  const nihilityCount = team.filter(m => {
    const kit = HSR_CHARACTER_KITS[m.characterId];
    const path = m.path || kit?.path;
    return path === "Nihility";
  }).length;

  const elationCount = team.filter(m => {
    const kit = HSR_CHARACTER_KITS[m.characterId];
    return (m.path || kit?.path) === "Elation";
  }).length;

  const ahaSpd = computeAhaSpd(team);

  const CASTORICE_ID = 'aa5d5fbc-0c17-466e-ac4f-8432092d1841';
  const hasCastoriceInTeam = team.some(m => m.characterId === CASTORICE_ID);

  const applyDamageToAlly = (member: TeamMember, damage: number, s: SimState, toughnessDamage: number = 0) => {
      if (member.isDowned) return;
      if (member.mooncocoon && damage > 0) {
          member.hp = 0;
          member.isDowned = true;
          member.mooncocoon = false;
          member.mooncocoon_expiry = false;
          return;
      }

      if (toughnessDamage > 0 && !member.is_broken) {
          const actualTghReduc = Math.min(member.toughness, toughnessDamage);
          member.toughness -= actualTghReduc;
          addLog(s, { type: 'event', message: `Reduced ${member.name}'s Toughness by ${actualTghReduc.toFixed(1)} (Remaining: ${member.toughness.toFixed(1)}/${member.max_toughness})` });
          
          if (member.toughness <= 0) {
              member.is_broken = true;
              addLog(s, { type: 'event', message: `[WEAKNESS BREAK] triggered on ${member.name}!` });
              // Simple delay for ally break (25%)
              const avInfo = s.avQueue.find(a => a.id === member.characterId && a.instanceId === "");
              if (avInfo) {
                  const spd = member.base_stats[CHAR_SPD_ID] || 100;
                  const delay = (10000 / spd) * 0.25;
                  avInfo.nextAV += delay;
                  addLog(s, { type: 'event', message: `${member.name}'s action delayed by ${delay.toFixed(1)} AV.` });
              }
          }
      }

      let remainingDamage = damage;

      // ── Multi-shield absorption ───────────────────────────────────────────
      // All active shields absorb the full damage amount simultaneously.
      // The effective shield is the highest-value one; overflow to HP = max(0, dmg ∁EmaxShield).
      if (member.activeShields && member.activeShields.length > 0) {
          const maxShieldBefore = Math.max(...member.activeShields.map(s => s.value));
          // Reduce every shield by the damage amount
          member.activeShields = member.activeShields.map(s => ({ ...s, value: s.value - remainingDamage }));
          // Remove broken shields  Esubtract each one's aggro contribution individually
          const broken: string[] = [];
          member.activeShields = member.activeShields.filter(sh => {
              if (sh.value <= 0) {
                  broken.push(sh.name);
                  member.aggroModifier -= (sh.aggroModifier ?? 0); // additive removal
                  return false;
              }
              return true;
          });
          if (broken.length > 0) {
              addLog(s, { type: 'event', message: `${member.name}'s shield(s) [${broken.join(', ')}] broke!` });
          }
          // Update the effective shield value (highest remaining, or 0)
          member.shield = member.activeShields.length > 0
              ? Math.max(...member.activeShields.map(s => s.value))
              : 0;
          // Overflow to HP = damage not covered by the best shield
          remainingDamage = Math.max(0, remainingDamage - maxShieldBefore);
      } else if (member.shield > 0) {
          // Legacy single-number shield (backward compat for kits that set member.shield directly)
          const shieldAbsorb = Math.min(member.shield, remainingDamage);
          member.shield -= shieldAbsorb;
          remainingDamage -= shieldAbsorb;
      }

      member.hp -= remainingDamage;
      if (member.hp <= 0) {
          // Mooncocoon (once per battle): saves ALL allies killed in the same action.
          // mooncocoonPendingTrigger is set on the first death and stays true for the
          // remainder of that action, so every subsequent ally killed in the same
          // action is also saved. After the action resolves, mooncocoonTriggered is
          // permanently set and mooncocoonPendingTrigger is cleared  Efuture actions
          // cannot trigger Mooncocoon again.
          if (s.hasCastoricePassive && (!s.mooncocoonTriggered || s.mooncocoonPendingTrigger)) {
              s.mooncocoonPendingTrigger = true;
              member.hp = 1;
              member.mooncocoon = true;
              member.mooncocoon_expiry = false;
          } else {
              member.hp = 0;
              member.isDowned = true;
              member.mooncocoon = false;
          }
      }
  };

  const applyToughnessDamage = (attacker: TeamMember, target: SimEnemy, baseToughness: number, s: SimState, ignoreWeakness: boolean = false) => {
      if (target.hp <= 0) return;
      if (target.is_broken) return;

      const isWeak = ignoreWeakness || target.weaknesses.includes(attacker.element);
      if (!isWeak) return;

      const reduction = calculateToughnessReduction(
          baseToughness,
          0,
          0,
          attacker.buffs.break_efficiency || 0,
          0,
          1.0
      );
      const actualReduction = Math.min(target.toughness, reduction);
      target.toughness -= actualReduction;

      addLog(s, { type: 'event', message: `Reduced ${target.name}'s Toughness by ${actualReduction.toFixed(1)} (Remaining: ${target.toughness.toFixed(1)}/${target.max_toughness})` });

      if (target.toughness > 0) return;

      // Toughness bar depleted  Edeal Break DMG (always ÁE.9, enemy not yet Weakness Broken).
      const breakEffect = attacker.base_stats[CHAR_BE_ID] || 0;
      const breakDmg = calculateBreakDamage({
          character: attacker,
          lightcone: attacker.lightcone,
          enemy: target,
          ability_multiplier: 1.0,
          scaling_stat_id: ATK_ID
      }, breakEffect, target.max_toughness);

      target.hp = Math.max(0, Math.floor(target.hp - breakDmg));
      s.totalDamage += breakDmg;
      addLog(s, { type: 'event', message: `Break DMG on ${target.name} -> ${breakDmg.toLocaleString()} (HP: ${target.hp}/${target.max_hp})` });

      // Check for additional toughness bars (intermediate bar  EBreak DMG only, no other effects).
      if (target.extra_toughness_bars && target.extra_toughness_bars.length > 0) {
          const nextBar = target.extra_toughness_bars.shift()!;
          const nextBarMax = target.max_extra_toughness_bars.shift()!;
          target.toughness = nextBar;
          target.max_toughness = nextBarMax;
          addLog(s, { type: 'event', message: `[INTERMEDIATE BREAK] ${target.name}: next Toughness bar activated (${target.toughness}/${target.max_toughness}). No action delay or debuff.` });
          return;
      }

      // Final bar depleted  Efull Weakness Break.
      target.is_broken = true;
      addLog(s, { type: 'event', message: `[WEAKNESS BREAK] ${target.name} broken via ${attacker.element}!` });

      // Action delay: 25% of the enemy's action value.
      const avInfo = s.avQueue.find(a => a.instanceId === target.instanceId && a.id === target.id);
      if (avInfo) {
          const spd = target.base_stats[ENEMY_SPD_ID] || 100;
          const delay = (10000 / spd) * 0.25;
          avInfo.nextAV += delay;
          addLog(s, { type: 'event', message: `${target.name}'s action delayed by ${delay.toFixed(1)} AV (25%).` });
      }

      // Type-specific debuff (150% base chance  Etreated as guaranteed in simulation).
      const debuffName = BREAK_DEBUFF_BY_ELEMENT[attacker.element];
      if (debuffName) {
          const isNormal = target.tier === 'normal';
          const initialStacks = attacker.element === "Wind"
              ? (WIND_SHEAR_INITIAL_STACKS[target.tier] ?? 1)
              : attacker.element === "Quantum"
              ? ENTANGLEMENT_INITIAL_STACKS
              : 1;

          const existing = target.activeDebuffs[debuffName];
          if (!existing) {
              target.activeDebuffs[debuffName] = {
                  duration:               BREAK_DEBUFF_DURATION[debuffName] ?? 2,
                  stacks:                 initialStacks,
                  attacker_level:         attacker.level,
                  attacker_break_effect:  attacker.base_stats[CHAR_BE_ID] || 0,
                  max_toughness_at_break: target.max_toughness,
                  max_hp_at_break:        target.max_hp,
                  is_normal_enemy:        isNormal,
              };
              target.debuffCount++;
              addLog(s, { type: 'event', message: `Applied [${debuffName}] (${initialStacks} stack(s)) to ${target.name}.` });
          } else if (attacker.element === "Wind") {
              // Wind Shear: re-applying refreshes duration and stacks up to max
              existing.stacks = Math.min(WIND_SHEAR_MAX_STACKS, (existing.stacks ?? 0) + initialStacks);
              existing.duration = BREAK_DEBUFF_DURATION["Wind Shear"];
              addLog(s, { type: 'event', message: `[Wind Shear] refreshed on ${target.name} (${existing.stacks} stacks).` });
          }

          // Imprisonment: apply SPD -10% immediately
          if (attacker.element === "Imaginary") {
              const breakEffect = attacker.base_stats[CHAR_BE_ID] || 0;
              const actionDelay = 0.30 * (1 + breakEffect / 100);
              const avInfo = s.avQueue.find(a => a.instanceId === target.instanceId && a.id === target.id);
              if (avInfo) {
                  const spd = target.base_stats[ENEMY_SPD_ID] || 100;
                  const delay = (10000 / spd) * actionDelay;
                  avInfo.nextAV += delay;
                  addLog(s, { type: 'event', message: `[Imprisonment] delays ${target.name}'s action by ${delay.toFixed(1)} AV and reduces SPD by 10%.` });
              }
          }

          // Entanglement: apply action delay
          if (attacker.element === "Quantum") {
              const breakEffect = attacker.base_stats[CHAR_BE_ID] || 0;
              const actionDelay = 0.20 * (1 + breakEffect / 100);
              const avInfo = s.avQueue.find(a => a.instanceId === target.instanceId && a.id === target.id);
              if (avInfo) {
                  const spd = target.base_stats[ENEMY_SPD_ID] || 100;
                  const delay = (10000 / spd) * actionDelay;
                  avInfo.nextAV += delay;
                  addLog(s, { type: 'event', message: `[Entanglement] delays ${target.name}'s action by ${delay.toFixed(1)} AV.` });
              }
          }

          s.team.forEach(m => {
              const k = HSR_CHARACTER_KITS[m.characterId];
              if (k?.hooks?.onGlobalDebuff) k.hooks.onGlobalDebuff(s, attacker, target);
          });
      }
  };

  const checkMooncocoonRecovery = (member: TeamMember, s: SimState) => {
    if (member.mooncocoon && (member.hp > 1.1 || member.shield > 0)) {
        member.mooncocoon = false;
        member.mooncocoon_expiry = false;
        member.loggedMooncocoon = false;
        addLog(s, { type: 'event', message: `${member.name}'s [Mooncocoon] removed. Survival confirmed.` });
    }
  };

  /**
   * Tick all break DoTs on an enemy at the start of its turn (before recovery).
   * DoTs are: Bleed, Burn, Shock, Wind Shear, Entanglement.
   * Freeze deals its damage once on the tick and skips the enemy's turn.
   * Imprisonment and Entanglement delays were already applied on break; no extra tick DMG.
   */
  const tickBreakDoTs = (enemy: SimEnemy, s: SimState): boolean => {
      let isFrozen = false;
      const DOT_NAMES = ["Bleed", "Burn", "Shock", "Wind Shear", "Entanglement", "Freeze"];

      for (const debuffName of DOT_NAMES) {
          const debuff = enemy.activeDebuffs[debuffName];
          if (!debuff) continue;

          const hasDoTDamage = debuffName !== "Imprisonment";
          if (hasDoTDamage && debuffName !== "Freeze") {
              // Determine element RES for this DoT's element type
              const elementForDebuff: Record<string, string> = {
                  "Bleed": "Physical", "Burn": "Fire", "Shock": "Lightning",
                  "Wind Shear": "Wind", "Entanglement": "Quantum",
              };
              const dotElement = elementForDebuff[debuffName] ?? "Physical";
              const baseRes = enemy.elemental_res[dotElement] ?? enemy.resistance;

              const dotDmg = calculateBreakDoTDamage({
                  debuffName,
                  attackerLevel:    debuff.attacker_level    ?? 80,
                  breakEffect:      debuff.attacker_break_effect ?? 0,
                  breakDmgIncrease: 0,
                  defIgnore:  0,
                  defReduction: 0,
                  resPen: 0,
                  enemyLevel:   enemy.level,
                  baseRes,
                  vulnerability:   enemy.vulnerability,
                  dmgReduction:    enemy.dmg_reduction,
                  isBroken:        enemy.is_broken,
                  stacks:          debuff.stacks,
                  maxToughness:    debuff.max_toughness_at_break,
                  maxHp:           debuff.max_hp_at_break,
                  isNormalEnemy:   debuff.is_normal_enemy,
              });

              enemy.hp = Math.max(0, enemy.hp - dotDmg);
              s.totalDamage += dotDmg;
              addLog(s, { type: 'event', message: `[${debuffName}] ticks on ${enemy.name} -> ${dotDmg.toLocaleString()} DMG (HP: ${enemy.hp}/${enemy.max_hp})` });
          }

          if (debuffName === "Freeze") {
              isFrozen = true;
              // Deal Freeze DMG once on the tick
              const freezeDmg = calculateBreakDoTDamage({
                  debuffName: "Freeze",
                  attackerLevel:    debuff.attacker_level    ?? 80,
                  breakEffect:      debuff.attacker_break_effect ?? 0,
                  breakDmgIncrease: 0,
                  defIgnore: 0, defReduction: 0, resPen: 0,
                  enemyLevel:    enemy.level,
                  baseRes:       enemy.elemental_res["Ice"] ?? enemy.resistance,
                  vulnerability: enemy.vulnerability,
                  dmgReduction:  enemy.dmg_reduction,
                  isBroken:      enemy.is_broken,
              });
              enemy.hp = Math.max(0, enemy.hp - freezeDmg);
              s.totalDamage += freezeDmg;
              addLog(s, { type: 'event', message: `[Freeze] deals ${freezeDmg.toLocaleString()} DMG to ${enemy.name} and skips its turn.` });

              // Unfreeze: advance next turn by 50% (reduce AV gap)
              const avInfo = s.avQueue.find(a => a.instanceId === enemy.instanceId && a.id === enemy.id);
              if (avInfo) {
                  const spd = enemy.base_stats[ENEMY_SPD_ID] || 100;
                  const fullAV = 10000 / spd;
                  avInfo.nextAV = s.currentAV + fullAV * 0.5;
                  addLog(s, { type: 'event', message: `[Freeze] expires  E${enemy.name}'s next action advanced by 50%.` });
              }

              delete enemy.activeDebuffs["Freeze"];
              if (enemy.debuffCount > 0) enemy.debuffCount--;
          }
      }

      // Tick durations (non-Freeze  EFreeze was already deleted above)
      for (const key of Object.keys(enemy.activeDebuffs)) {
          const eff = enemy.activeDebuffs[key];
          if (eff.duration > 0) eff.duration--;
          if (eff.duration <= 0) {
              delete enemy.activeDebuffs[key];
              if (enemy.debuffCount > 0) enemy.debuffCount--;
          }
      }

      return isFrozen;
  };

  const checkAllies = (s: SimState) => {
    s.team.forEach(member => {
        if (member.isDowned && !member.loggedDowned) {
            member.loggedDowned = true;
            if (member.mooncocoon === false && member.loggedMooncocoon) {
                addLog(s, { type: 'event', message: `${member.name}'s [Mooncocoon] collapsed due to further damage! Character is downed.` });
            } else {
                addLog(s, { type: 'event', message: `${member.name} has been downed!` });
            }
        } else if (member.mooncocoon && !member.loggedMooncocoon) {
            member.loggedMooncocoon = true;
            addLog(s, { type: 'event', message: `${member.name} entered [Mooncocoon]! Survival guaranteed until next turn.` });
        }
    });
  };

  let state: SimState = {
    team,
    enemies: [],
    enemy: null as any,
    enemyPool: [],
    waves,
    currentWaveIndex: 0,
    currentAV: 0,
    maxAV,
    skillPoints: 3,
    totalDamage: 0,
    logs: [],
    addLog: options.skipLogs ? () => {} : (entry) => addLog(state, entry),
    stacks: {},
    buffDurations: {},
    turnCounters: {},
    avQueue: [],
    nihilityCount,
    hasCastoricePassive: hasCastoriceInTeam || !!options.hasCastorice,
    mooncocoonTriggered: false,
    mooncocoonPendingTrigger: false,
    elationCount,
    punchline: elationCount,
    certifiedBangerState: {},
    // ── Extra Turns / Follow-Up / Summon state ─────────────────────────────
    summons: [],
    pendingFollowUps: [],
    pendingExtraTurns: [],
    isExtraTurn: false,
    // ── Zones / Territory state ─────────────────────────────────────────────
    activeZones: [],
    activeTerritory: null,
    // Methods are wired after `state` is declared (closures need `state` ref)
    queueFollowUp: (_fup: PendingFollowUp) => { /* wired below */ },
    grantExtraTurn: (_id: string, _iid: string) => { /* wired below */ },
    summonUnit: (_s: SummonUnit) => { /* wired below */ },
    dismissSummon: (_iid: string) => { /* wired below */ },
    selectCombatTarget: (_isBounce?: boolean): CombatTarget | null => null, // wired below
    activateZone: (_z: ZoneEffect) => { /* wired below */ },
    deactivateZone: (_id: string) => { /* wired below */ },
    activateTerritory: (_t: TerritoryEffect): boolean => false, // wired below
    deactivateTerritory: () => { /* wired below */ },
    applyHeal: (_t, _a) => { /* wired below */ },
    applyShield: (_t, _s) => { /* wired below */ },
    applyDamageToAlly: (m, d) => applyDamageToAlly(m, d, state),
    checkEnemies: () => checkEnemies(state),
    checkAllies: () => checkAllies(state),
    selectTarget: (isBounce = false) => selectAggroTarget(state.team, isBounce)
  };

  // ── Wire state methods (need `state` to be declared first) ──────────────────
  state.queueFollowUp = (fup: PendingFollowUp) => {
    state.pendingFollowUps.push(fup);
  };
  state.grantExtraTurn = (actorId: string, actorInstanceId: string, opts?: { isLowPriority?: boolean; reason?: string }) => {
    state.pendingExtraTurns.push({
      actorId,
      actorInstanceId,
      isLowPriority: opts?.isLowPriority ?? false,
      reason: opts?.reason,
    });
  };
  state.summonUnit = (summon: SummonUnit) => {
    // Remove any existing summon with the same instanceId first
    state.summons = state.summons.filter(s => s.instanceId !== summon.instanceId);
    state.avQueue  = state.avQueue.filter(a => a.instanceId !== summon.instanceId);
    // Ensure new buff/shield fields are present
    if (!summon.activeShields) summon.activeShields = [];
    summon.buffs.outgoing_healing_boost    ??= 0;
    summon.buffs.incoming_healing_boost    ??= 0;
    summon.buffs.incoming_healing_reduction ??= 0;
    summon.buffs.shield_bonus              ??= 0;
    state.summons.push(summon);
    if (!summon.zeroSpd && summon.spd > 0) {
      state.avQueue.push({ id: summon.id, instanceId: summon.instanceId, nextAV: state.currentAV + 10000 / summon.spd });
    }
    addLog(state, { type: 'info', message: `${summon.name} [${summon.kind}] summoned by ${team.find(m => m.characterId === summon.masterId)?.name ?? summon.masterId} (SPD=${summon.spd}).` });
  };
  state.dismissSummon = (instanceId: string) => {
    const s = state.summons.find(u => u.instanceId === instanceId);
    if (s) addLog(state, { type: 'info', message: `${s.name} [${s.kind}] dismissed.` });
    state.summons = state.summons.filter(u => u.instanceId !== instanceId);
    state.avQueue = state.avQueue.filter(a => a.instanceId !== instanceId);
  };
  state.selectCombatTarget = (isBounce = false): CombatTarget | null =>
    selectCombatTargetFn(state.team, state.summons, isBounce);

  // ── Healing / Shield helpers ───────────────────────────────────────────────
  state.applyHeal = (target: TeamMember | SummonUnit, amount: number) => {
    const asMember = target as TeamMember;
    if (asMember.isDowned) return;
    const prev = target.hp;
    target.hp = Math.min(target.max_hp, target.hp + amount);
    const actual = target.hp - prev;
    if (actual > 0) {
      const name = (target as any).name || (target as any).id || 'Unknown';
      addLog(state, { type: 'event', message: `${name} healed for ${actual.toFixed(0)} HP (${target.hp.toFixed(0)}/${target.max_hp.toFixed(0)}).` });
    }
  };

  state.applyShield = (target: TeamMember | SummonUnit, shield: ShieldInstance) => {
    if (!target.activeShields) target.activeShields = [];
    // Remove any existing instance with the same name (subtract its aggro contribution first)
    const existing = target.activeShields.find(s => s.name === shield.name);
    if (existing && (target as TeamMember).aggroModifier !== undefined) {
      (target as TeamMember).aggroModifier -= (existing.aggroModifier ?? 0);
    }
    const replaced = !!existing;
    target.activeShields = target.activeShields.filter(s => s.name !== shield.name);
    target.activeShields.push(shield);
    // Additively apply the new shield's aggro contribution
    if ((target as TeamMember).aggroModifier !== undefined) {
      (target as TeamMember).aggroModifier += (shield.aggroModifier ?? 0);
    }
    // Effective shield = highest value
    target.shield = Math.max(...target.activeShields.map(s => s.value));
    const name = (target as any).name || (target as any).id || 'Unknown';
    addLog(state, { type: 'event', message: `${name} ${replaced ? 'refreshed' : 'gained'} [${shield.name}] shield: ${shield.value.toFixed(0)} HP (${shield.duration === -1 ? 'permanent' : shield.duration + ' turns'}).` });
  };

  // ── Zone / Territory helpers ───────────────────────────────────────────────
  state.activateZone = (zone: ZoneEffect) => {
    state.activeZones.push(zone);
    const owner = team.find(m => m.characterId === zone.ownerId);
    addLog(state, { type: 'info', message: `[ZONE] ${zone.name} activated by ${owner?.name ?? zone.ownerId} (${zone.duration} turns). Affects: ${[zone.affectsAllies && 'allies', zone.affectsEnemies && 'enemies'].filter(Boolean).join(', ')}.` });
    team.forEach(m => {
      if (!m.isDowned) {
        const k = HSR_CHARACTER_KITS[m.characterId];
        if (k?.hooks?.onZoneActivated) k.hooks.onZoneActivated(state, m, zone);
      }
    });
  };

  state.deactivateZone = (instanceId: string) => {
    const zone = state.activeZones.find(z => z.instanceId === instanceId);
    if (zone) {
      addLog(state, { type: 'info', message: `[ZONE] ${zone.name} expired.` });
      state.activeZones = state.activeZones.filter(z => z.instanceId !== instanceId);
    }
  };

  state.activateTerritory = (territory: TerritoryEffect): boolean => {
    if (state.activeTerritory) {
      addLog(state, { type: 'event', message: `[TERRITORY] Cannot activate ${territory.name}  E${state.activeTerritory.name} is already active.` });
      return false;
    }
    state.activeTerritory = territory;
    const owner = team.find(m => m.characterId === territory.ownerId);
    addLog(state, { type: 'info', message: `[TERRITORY] ${territory.name} activated by ${owner?.name ?? territory.ownerId} (${territory.duration} turns). Affects: ${[territory.affectsAllies && 'allies', territory.affectsEnemies && 'enemies'].filter(Boolean).join(', ')}.` });
    team.forEach(m => {
      if (!m.isDowned) {
        const k = HSR_CHARACTER_KITS[m.characterId];
        if (k?.hooks?.onTerritoryActivated) k.hooks.onTerritoryActivated(state, m, territory);
      }
    });
    return true;
  };

  state.deactivateTerritory = () => {
    if (state.activeTerritory) {
      addLog(state, { type: 'info', message: `[TERRITORY] ${state.activeTerritory.name} expired.` });
      state.activeTerritory = null;
    }
  };

  // ── Aha Instant handler ────────────────────────────────────────────────────
  const handleAhaTurn = () => {
    const currentPunchline = state.punchline ?? 0;
    const aliveElation = team.filter(m => {
      const kit = HSR_CHARACTER_KITS[m.characterId];
      return !m.isDowned && m.hp > 0 && (m.path || kit?.path) === "Elation";
    });

    addLog(state, {
      type: 'turn',
      message: `Aha's turn starts. (SPD=${ahaSpd.toFixed(1)}, Punchline=${currentPunchline})`,
      actor: { id: 'aha', instanceId: 'aha', name: 'Aha', type: 'ally', color: '#d946ef' }
    });

    if (aliveElation.length > 0) {
      // ── Aha Instant ──────────────────────────────────────────────────────
      addLog(state, { type: 'info', message: `[AHA INSTANT] ${aliveElation.length} Elation character(s) will use their Elation Skills.` });

      // Remove Entanglement, Imprisonment, Freeze from all Elation chars
      aliveElation.forEach(m => {
        ["Entanglement", "Imprisonment", "Freeze"].forEach(cc => {
          if (m.activeDebuffs[cc]) {
            delete m.activeDebuffs[cc];
            if (m.debuffCount !== undefined) (m as any).debuffCount = Math.max(0, (m as any).debuffCount - 1);
            addLog(state, { type: 'event', message: `[Aha Instant] ${m.name}'s ${cc} removed.` });
          }
        });
      });

      // Sort by SPD descending  Efastest Elation char uses skill first
      const sorted = [...aliveElation].sort((a, b) =>
        (b.base_stats[CHAR_SPD_ID] || 100) - (a.base_stats[CHAR_SPD_ID] || 100)
      );

      const ahaTarget = state.enemies.find(e => e && e.hp > 0);

      sorted.forEach(m => {
        if (!ahaTarget || ahaTarget.hp <= 0) return;
        const kit = HSR_CHARACTER_KITS[m.characterId];

        // Determine punchline points: use Certified Banger if active, else team Punchline
        const cbInstances = state.certifiedBangerState?.[m.characterId] ?? [];
        const cbTotalPoints = cbInstances.reduce((s, cb) => s + (cb.duration > 0 ? cb.points : 0), 0);
        const hasCB = cbTotalPoints > 0;
        const punchlinePoints = hasCB ? cbTotalPoints : currentPunchline;

        addLog(state, {
          type: 'action',
          message: `${m.name} uses Elation Skill [Aha Instant] (PunchlinePts=${punchlinePoints})`,
          actor: { id: m.characterId, name: m.name || m.characterId, type: 'ally', color: getActorColor(m.characterId, "", 'ally') }
        });

        if (kit?.hooks?.onElationSkill) {
          // Kit provides a custom implementation
          kit.hooks.onElationSkill(state, m, punchlinePoints);
        } else {
          // Generic: use elation_skill scaling from kit, or fall back to a 1.0 multiplier
          const rawAbility = kit?.abilities?.elation_skill ?? null;
          const slotName = kit?.slot_names?.elation_skill ?? 'Elation Skill';
          const scalingConfig = rawAbility ? getPrimaryScalingConfig(rawAbility) : { default_multiplier: 1.0, stat_id: ATK_ID };
          const abilityMultiplier = rawAbility
            ? resolveDatabaseScaling(m.databaseAbilities || [], slotName, m.abilityLevels.talent || 1, scalingConfig)
            : 1.0;

          const elationStat  = m.base_stats[ELATION_STAT_ID]  || 0;
          const merrymakeStat = m.base_stats[MERRYMAKE_STAT_ID] || 0;
          const cr = (m.base_stats[CRIT_RATE_ID] || 5) + m.buffs.crit_rate;
          const cd = (m.base_stats[CRIT_DMG_ID] || 50) + m.buffs.crit_dmg;

          const elationDmg = calculateElationDamage({
            attackerLevel: m.level,
            element: m.element,
            elation: elationStat,
            merrymake: merrymakeStat,
            punchlinePoints,
            defIgnore:     (m.buffs.def_ignore    || 0) / 100,
            defReduction:  (m.buffs.def_reduction  || 0) / 100,
            resPen:        (m.buffs.res_pen        || 0) / 100,
            crit_rate: cr, crit_dmg: cd,
            abilityMultiplier,
            enemyLevel:   ahaTarget.level,
            elemental_res: ahaTarget.elemental_res,
            baseRes:       ahaTarget.resistance,
            is_broken:     ahaTarget.is_broken,
            vulnerability: ahaTarget.vulnerability,
            dmg_reduction: ahaTarget.dmg_reduction,
          });

          state.totalDamage += elationDmg;
          ahaTarget.hp = Math.max(0, Math.floor(ahaTarget.hp - elationDmg));
          addLog(state, { type: 'event', message: `[Elation DMG] on ${ahaTarget.name} -> ${elationDmg.toLocaleString()} (HP: ${ahaTarget.hp.toLocaleString()}/${ahaTarget.max_hp.toLocaleString()})` });
        }
      });

      checkEnemies(state);

      // Grant Certified Banger (current Punchline points, 2 turns) to each participating char
      if (currentPunchline > 0) {
        sorted.forEach(m => {
          if (!state.certifiedBangerState![m.characterId]) {
            state.certifiedBangerState![m.characterId] = [];
          }
          state.certifiedBangerState![m.characterId].push({ points: currentPunchline, duration: 2 });
          addLog(state, { type: 'event', message: `${m.name} gains [Certified Banger] (${currentPunchline} pts, 2 turns).` });
        });
      }

      // Consume all Punchlines
      state.punchline = 0;
      addLog(state, { type: 'info', message: `[Aha Instant] ended. All Punchlines consumed.` });

    } else {
      // ── Let There Be Laughter (no alive Elation chars) ───────────────────
      addLog(state, { type: 'info', message: `[LET THERE BE LAUGHTER]  ENo Elation characters. Aha launches up to 10 random hits.` });

      const ahaLevel = 80; // Aha acts at level 80 for LTBL

      const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
      const aliveAllies  = team.filter(m => !m.isDowned && m.hp > 0);
      type BFTarget = { kind: 'enemy'; e: SimEnemy } | { kind: 'ally'; m: TeamMember };
      const battlefield: BFTarget[] = [
        ...aliveEnemies.map(e  => ({ kind: 'enemy' as const, e })),
        ...aliveAllies.map(m   => ({ kind: 'ally'  as const, m })),
      ];

      for (let i = 0; i < 10; i++) {
        if (battlefield.length === 0) break;
        const hit = battlefield[Math.floor(Math.random() * battlefield.length)];
        if (!hit) continue;

        if (hit.kind === 'enemy') {
          const e = hit.e;
          if (e.hp <= 0) continue;
          const ltblDmg = calculateElationDamage({
            attackerLevel: ahaLevel, element: 'Quantum',
            elation: 0, merrymake: 0, punchlinePoints: 0,
            defIgnore: 0, defReduction: 0, resPen: 0,
            crit_rate: 0, crit_dmg: 0,
            abilityMultiplier: AHA_LTBL_HIT_MULTIPLIER,
            enemyLevel: e.level, elemental_res: e.elemental_res, baseRes: e.resistance,
            is_broken: e.is_broken, vulnerability: e.vulnerability, dmg_reduction: e.dmg_reduction,
          });
          e.hp = Math.max(0, Math.floor(e.hp - ltblDmg));
          state.totalDamage += ltblDmg;
          addLog(state, { type: 'event', message: `[Let There Be Laughter] Hit ${e.name} -> ${ltblDmg.toLocaleString()} Elation DMG (HP: ${e.hp}/${e.max_hp})` });
        } else {
          const m = hit.m;
          if (m.isDowned || m.hp <= 0) continue;
          // Allies take 1 DMG, non-fatal (minimum 1 HP remaining)
          m.hp = Math.max(1, m.hp - 1);
          addLog(state, { type: 'event', message: `[Let There Be Laughter] Hit ${m.name} -> 1 DMG (non-fatal, HP: ${m.hp}/${m.max_hp})` });
        }
      }
      checkEnemies(state);
      state.punchline = 0;
    }
  };

  // ── Follow-up / Extra Turn drain engine ───────────────────────────────────

  /** Execute the next pending follow-up (if any). Returns true if one was executed. */
  const executeNextFollowUp = (): boolean => {
    if (state.pendingFollowUps.length === 0) return false;
    const fup = state.pendingFollowUps.shift()!;

    const actor = team.find(m => m.characterId === fup.actorId);
    if (!actor || actor.isDowned || actor.hp <= 0) return true; // skip, still counts

    // Follow-up blocked by Crowd Control
    const hasCC = CC_DEBUFFS.some(cc => actor.activeDebuffs[cc]);
    if (hasCC) {
      addLog(state, { type: 'event', message: `${actor.name}'s follow-up blocked by crowd control.`, actor: { id: actor.characterId, name: actor.name || actor.characterId, type: 'ally', color: getActorColor(actor.characterId, "", 'ally') } });
      return true;
    }

    const fuTarget = fup.targetInstanceId
      ? state.enemies.find(e => e && e.instanceId === fup.targetInstanceId && e.hp > 0)
      : state.enemies.find(e => e && e.hp > 0);
    if (!fuTarget) return true;

    addLog(state, {
      type: 'action',
      message: `[FOLLOW-UP${fup.isCounter ? ' COUNTER' : ''}] ${actor.name} attacks.`,
      actor: { id: actor.characterId, name: actor.name || actor.characterId, type: 'ally', color: getActorColor(actor.characterId, "", 'ally') }
    });

    const fuAlive = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
    const fuIdx   = state.enemies.indexOf(fuTarget);

    let fuTargets: SimEnemy[];
    if (fup.action.targetType === 'AoE') {
      fuTargets = fuAlive;
    } else if (fup.action.targetType === 'Blast') {
      fuTargets = [fuTarget];
      const left  = state.enemies[fuIdx - 1];
      const right = state.enemies[fuIdx + 1];
      if (fuIdx > 0 && left && left.hp > 0) fuTargets.push(left);
      if (fuIdx < state.enemies.length - 1 && right && right.hp > 0) fuTargets.push(right);
    } else {
      fuTargets = [fuTarget];
    }

    fuTargets.forEach(t => {
      const result = calculateHsrDamage({
        character: actor,
        lightcone: actor.lightcone,
        enemy: t,
        ability_multiplier: fup.action.multiplier,
        scaling_stat_id: fup.action.stat_id,
      });
      state.totalDamage += result.expected_dmg;
      t.hp = Math.max(0, Math.floor(t.hp - result.expected_dmg));
      addLog(state, { type: 'event', message: `Follow-up hit on ${t.name} -> ${result.expected_dmg.toLocaleString()} DMG (HP: ${t.hp.toLocaleString()}/${t.max_hp.toLocaleString()})` });
      const entangle = t.activeDebuffs["Entanglement"];
      if (entangle) entangle.stacks = Math.min(ENTANGLEMENT_MAX_STACKS, (entangle.stacks ?? 0) + 1);
      if (fup.action.toughness_damage) applyToughnessDamage(actor, t, fup.action.toughness_damage, state);
    });

    checkEnemies(state);

    // After a follow-up, check if other characters want to queue their own follow-ups
    team.forEach(m => {
      if (!m.isDowned) {
        const k = HSR_CHARACTER_KITS[m.characterId];
        if (k?.hooks?.onCheckFollowUp) k.hooks.onCheckFollowUp(state, m, 'after_follow_up');
      }
    });
    return true;
  };

  /** Execute a pending extra turn for a character or summon. */
  const executeExtraTurn = (et: PendingExtraTurn) => {
    // Summon extra turn?
    const summonActor = state.summons.find(s => s.id === et.actorId && s.instanceId === et.actorInstanceId);
    if (summonActor) {
      const master = team.find(m => m.characterId === summonActor.masterId);
      if (!master || master.isDowned) return;
      const masterKit = HSR_CHARACTER_KITS[master.characterId];
      if (masterKit?.hooks?.onSummonTurn) {
        addLog(state, { type: 'turn', message: `${summonActor.name} [SUMMON EXTRA TURN]${et.reason ? `: ${et.reason}` : ''}`, actor: { id: summonActor.id, instanceId: summonActor.instanceId, name: summonActor.name, type: 'ally', color: getActorColor(master.characterId, "", 'ally') } });
        state.isExtraTurn = true;
        masterKit.hooks.onSummonTurn(state, master, summonActor);
        state.isExtraTurn = false;
      }
      return;
    }

    // Character extra turn
    const member = team.find(m => m.characterId === et.actorId);
    if (!member || member.isDowned) return;
    const kit = HSR_CHARACTER_KITS[member.characterId];
    if (!kit) return;

    addLog(state, {
      type: 'turn',
      message: `${member.name}'s EXTRA TURN${et.reason ? ` (${et.reason})` : ''}${et.isLowPriority ? ' [Low Priority]' : ''}.`,
      actor: { id: member.characterId, name: member.name || member.characterId, type: 'ally', color: getActorColor(member.characterId, "", 'ally') }
    });

    // Status effects do NOT tick during extra turns
    state.isExtraTurn = true;

    if (kit.hooks?.onExtraTurn) {
      kit.hooks.onExtraTurn(state, member);
    } else {
      // Default: basic or skill (based on SP). No ult.
      const etTarget = state.enemies.find(e => e && e.hp > 0);
      if (etTarget) {
        let etActionType: 'basic' | 'skill' = 'basic';
        if (state.skillPoints > 0) { etActionType = 'skill'; state.skillPoints--; }
        else { state.skillPoints++; }

        const etRaw    = etActionType === 'basic' ? kit.abilities.basic : kit.abilities.skill;
        const etSlot   = etActionType === 'basic' ? kit.slot_names.basic : kit.slot_names.skill;
        const etConfig = getPrimaryScalingConfig(etRaw);
        const etMult   = resolveDatabaseScaling(member.databaseAbilities || [], etSlot, member.abilityLevels[etActionType], etConfig);
        const etTType  = etConfig.targetType || 'SingleTarget';

        addLog(state, { type: 'action', message: `[EXTRA TURN] ${etActionType.toUpperCase()}: ${etSlot}`, actor: { id: member.characterId, name: member.name || member.characterId, type: 'ally', color: getActorColor(member.characterId, "", 'ally') } });

        const etAlive = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
        const etIdx   = state.enemies.indexOf(etTarget);
        let etTargets: SimEnemy[];
        if (etTType === 'AoE') { etTargets = etAlive; }
        else if (etTType === 'Blast') {
          etTargets = [etTarget];
          const l = state.enemies[etIdx - 1]; const r = state.enemies[etIdx + 1];
          if (etIdx > 0 && l && l.hp > 0) etTargets.push(l);
          if (etIdx < state.enemies.length - 1 && r && r.hp > 0) etTargets.push(r);
        } else { etTargets = [etTarget]; }

        etTargets.forEach(t => {
          const res = calculateHsrDamage({ character: member, lightcone: member.lightcone, enemy: t, ability_multiplier: etMult, scaling_stat_id: etConfig.stat_id });
          state.totalDamage += res.expected_dmg;
          t.hp = Math.max(0, Math.floor(t.hp - res.expected_dmg));
          addLog(state, { type: 'event', message: `Hit on ${t.name} -> ${res.expected_dmg.toLocaleString()} DMG (HP: ${t.hp}/${t.max_hp})` });
          const entangle = t.activeDebuffs["Entanglement"];
          if (entangle) entangle.stacks = Math.min(ENTANGLEMENT_MAX_STACKS, (entangle.stacks ?? 0) + 1);
          if (etConfig.toughness_damage) applyToughnessDamage(member, t, etConfig.toughness_damage, state);
        });

        checkEnemies(state);
        const etAction: Action = { type: etActionType, multiplier: etMult, stat_id: etConfig.stat_id, targetType: etTType };
        if (kit.hooks?.onAfterAction) kit.hooks.onAfterAction(state, member, etAction, etTarget);
        team.forEach(m => { if (!m.isDowned) { const k = HSR_CHARACTER_KITS[m.characterId]; if (k?.hooks?.onCheckFollowUp) k.hooks.onCheckFollowUp(state, m, 'after_extra_turn'); } });
      }
    }

    state.isExtraTurn = false;
  };

  /**
   * Drain all pending follow-up actions and normal-priority extra turns, in priority order:
   *   Follow-ups (highest) ↁENormal Extra Turns ↁE[loop] ↁELow-Priority Extra Turns (lowest)
   *
   * Called after every regular action, ult, and enemy action.
   */
  const drainPendingActions = () => {
    let guard = 0;
    // Phase 1: follow-ups and normal extra turns (interleaved  Efollow-ups drain after each extra turn)
    while (guard++ < 50) {
      if (executeNextFollowUp()) continue; // always drain FUPs first
      const normalIdx = state.pendingExtraTurns.findIndex(e => !e.isLowPriority);
      if (normalIdx !== -1) {
        const [et] = state.pendingExtraTurns.splice(normalIdx, 1);
        if (et) executeExtraTurn(et);
        continue; // re-check for FUPs triggered by the extra turn
      }
      break; // nothing left in high-priority queue
    }
    // Phase 2: low-priority extra turns (after all normal extra turns and follow-ups)
    guard = 0;
    while (state.pendingExtraTurns.length > 0 && guard++ < 20) {
      const et = state.pendingExtraTurns.shift();
      if (et) {
        executeExtraTurn(et);
        // Drain any follow-ups triggered by the low-priority extra turn
        let fupGuard = 0;
        while (executeNextFollowUp() && fupGuard++ < 20) { /* drain */ }
      }
    }
  };

  addLog(state, { type: 'header', message: 'BATTLE START' });
  addLog(state, { type: 'info', message: `Nihility Count: ${nihilityCount}` });
  if (state.hasCastoricePassive) addLog(state, { type: 'info', message: `Global Passive: Mooncocoon (Castorice) is ACTIVE` });
  
  setupWave(state, 0);
  state.enemy = state.enemies.find(e => e !== null) as SimEnemy;

  team.forEach(m => {
    state.buffDurations[m.characterId] = { ...m.activeBuffs };
    const kit = HSR_CHARACTER_KITS[m.characterId];
    if (kit) {
      if (kit.special_modifiers.stat_boosts) {
        const boosts = kit.special_modifiers.stat_boosts({ ...m.buffs, eidolon: m.eidolon });
        if (boosts.atk_percent) m.buffs.atk_percent += boosts.atk_percent;
        if (boosts.crit_rate) m.buffs.crit_rate += boosts.crit_rate;
        if (boosts.crit_dmg) m.buffs.crit_dmg += boosts.crit_dmg;
        if (boosts.dmg_boost) m.buffs.dmg_boost += boosts.dmg_boost;
      }
      if (kit.hooks?.onBattleStart) kit.hooks.onBattleStart(state, m);
    }
    const spd = m.base_stats[CHAR_SPD_ID] || 100;
    const atk = (m.base_stats[ATK_ID] || 0) + (m.lightcone.base_stats[ATK_ID] || 0);
    const def = (m.base_stats[DEF_ID] || 0) + (m.lightcone.base_stats[DEF_ID] || 0);
    const cr = (m.base_stats[CRIT_RATE_ID] || 5) + m.buffs.crit_rate;
    const cd = (m.base_stats[CRIT_DMG_ID] || 50) + m.buffs.crit_dmg;
    
    const aggro = computeAggro(m);
    addLog(state, {
        type: 'info',
        message: `${m.name}: ${m.element}, Path=${m.path ?? 'Unknown'}, Aggro=${aggro.toFixed(1)}, HP=${m.hp.toFixed(0)}, ATK=${atk.toFixed(0)}, DEF=${def.toFixed(0)}, SPD=${spd.toFixed(1)}, CR=${cr.toFixed(1)}%, CD=${cd.toFixed(1)}%`,
        actor: {
            id: m.characterId,
            name: m.name || m.characterId,
            type: 'ally',
            color: getActorColor(m.characterId, "", 'ally')
        }
    });
    
    state.avQueue.push({ id: m.characterId, instanceId: "", nextAV: 10000 / spd });
  });

  // ── Elation / Aha Instant setup ──────────────────────────────────────────
  if (elationCount > 0) {
    addLog(state, { type: 'info', message: `Elation Count: ${elationCount}  EAha SPD=${ahaSpd.toFixed(1)}, Initial Punchline=${elationCount}` });

    // All Elation chars start with Certified Banger (20 pts, 2 turns)
    team.forEach(m => {
      const kit = HSR_CHARACTER_KITS[m.characterId];
      if ((m.path || kit?.path) === "Elation") {
        state.certifiedBangerState![m.characterId] = [{ points: 20, duration: 2 }];
        addLog(state, { type: 'info', message: `${m.name} gains initial [Certified Banger] (20 pts, 2 turns) at battle start.` });
      }
    });

    // Add Aha to the action queue (punchline starts > 0, so Aha is present)
    state.avQueue.push({ id: 'aha', instanceId: 'aha', nextAV: 10000 / ahaSpd });
  }

  addLog(state, { type: 'header', message: 'COMBAT START' });

  let isDefeated = false;

  while (state.currentAV <= state.maxAV) {
    const allAlliesDowned = team.every(m => m.isDowned);
    if (allAlliesDowned) {
        addLog(state, { type: 'defeat', message: 'ALL ALLIES DOWNED!' });
        isDefeated = true;
        break;
    }

    const allEnemiesDead = state.enemies.every(e => !e || e.hp <= 0);
    const poolEmpty = state.enemyPool.length === 0;

    if (allEnemiesDead && poolEmpty) {
        if (state.currentWaveIndex < state.waves.length - 1) {
            setupWave(state, state.currentWaveIndex + 1);
            continue; 
        } else {
            addLog(state, { type: 'victory', message: 'ALL WAVES CLEARED!' });
            break;
        }
    }

    state.avQueue.sort((a, b) => a.nextAV - b.nextAV);
    const actorInfo = state.avQueue[0];
    if (!actorInfo) break; // Safety: queue exhausted (should not happen with alive allies)
    state.currentAV = actorInfo.nextAV;
    if (state.currentAV > state.maxAV) break;

    const allyIndex = team.findIndex(m => m.characterId === actorInfo.id);
    if (allyIndex !== -1) {
      const member = team[allyIndex];

      if (member.is_broken) {
          member.is_broken = false;
          member.toughness = member.max_toughness;
          addLog(state, { type: 'event', message: `${member.name} recovered from Weakness Break. Toughness restored.` });
      }

      if (member.mooncocoon) {
          if (member.mooncocoon_expiry) {
              if (member.hp > 1.1 || member.shield > 0) {
                  member.mooncocoon = false;
                  member.mooncocoon_expiry = false;
                  addLog(state, { type: 'event', message: `${member.name}'s [Mooncocoon] removed. Survival confirmed.` });
              } else {
                  member.hp = 0;
                  member.isDowned = true;
                  member.loggedDowned = true;
                  member.mooncocoon = false;
                  member.mooncocoon_expiry = false;
                  member.loggedMooncocoon = false;
                  addLog(state, { type: 'event', message: `${member.name} failed to recover. [Mooncocoon] collapsed. Character is downed.` });
              }
          } else {
              member.mooncocoon_expiry = true;
              addLog(state, { type: 'event', message: `${member.name} is acting while in [Mooncocoon].` });
          }
      }

      if (member.isDowned) {
          // Advance AV so downed allies don't block the queue, but keep them in it
          // in case a revival mechanic restores them later.
          actorInfo.nextAV += 10000 / (member.base_stats[CHAR_SPD_ID] || 100);
          continue;
      }

      // ── Elation: generate Punchline & tick Certified Banger ──────────────
      const memberPath = member.path || HSR_CHARACTER_KITS[member.characterId]?.path;
      if (elationCount > 0 && memberPath === "Elation") {
          state.punchline = (state.punchline ?? 0) + 1;
          addLog(state, { type: 'event', message: `${member.name} generates 1 Punchline (Total: ${state.punchline})` });

          // Re-add Aha if not already queued (punchline > 0 ↁEAha appears)
          const ahaQueued = state.avQueue.some(a => a.id === 'aha');
          if (!ahaQueued) {
              state.avQueue.push({ id: 'aha', instanceId: 'aha', nextAV: state.currentAV + 10000 / ahaSpd });
              addLog(state, { type: 'info', message: `Aha reappears in the action bar (Punchline: ${state.punchline})` });
          }
      }

      // Tick Certified Banger durations (at turn start, like other buffs)
      if (state.certifiedBangerState?.[member.characterId]) {
          state.certifiedBangerState[member.characterId] = state.certifiedBangerState[member.characterId]
              .map(cb => ({ ...cb, duration: cb.duration - 1 }))
              .filter(cb => cb.duration > 0);
      }

      const kit = HSR_CHARACTER_KITS[member.characterId];
      const spd = member.base_stats[CHAR_SPD_ID] || 100;
      const atk = (member.base_stats[ATK_ID] || 0) + (member.lightcone.base_stats[ATK_ID] || 0);
      const def = (member.base_stats[DEF_ID] || 0) + (member.lightcone.base_stats[DEF_ID] || 0);
      const cr = (member.base_stats[CRIT_RATE_ID] || 5) + member.buffs.crit_rate;
      const cd = (member.base_stats[CRIT_DMG_ID] || 50) + member.buffs.crit_dmg;
      const shieldStr = member.shield > 0 ? `, SHIELD=${member.shield.toFixed(0)}` : "";

      addLog(state, { 
          type: 'turn', 
          message: `${member.name || member.characterId}'s turn starts.`,
          actor: { 
              id: member.characterId, 
              name: member.name || member.characterId, 
              type: 'ally',
              color: getActorColor(member.characterId, "", 'ally')
          },
          subEntries: [
              `Stats: HP=${member.hp.toFixed(0)}/${member.max_hp.toFixed(0)}${shieldStr}, ATK=${atk.toFixed(0)}, DEF=${def.toFixed(0)}, SPD=${spd.toFixed(1)}, CR=${cr.toFixed(1)}%, CD=${cd.toFixed(1)}%`,
              `Active Buffs: ${formatStatus(state.buffDurations[member.characterId])}`,
              `Active Debuffs: ${formatStatus(member.activeDebuffs, true)}`
          ]
      });

      if (kit?.hooks?.onTurnStart) kit.hooks.onTurnStart(state, member);

      Object.entries(state.buffDurations[member.characterId]).forEach(([key, effect]) => {
          if (effect.duration > 0) effect.duration--;
          if (effect.duration <= 0) delete state.buffDurations[member.characterId][key];
      });
      Object.entries(member.activeDebuffs).forEach(([key, effect]) => {
          if (effect.duration > 0) effect.duration--;
          if (effect.duration <= 0) delete member.activeDebuffs[key];
      });

      // ── Tick active shield durations ──────────────────────────────────────
      if (member.activeShields && member.activeShields.length > 0) {
          const expired: string[] = [];
          member.activeShields = member.activeShields
              .map(s => s.duration === -1 ? s : { ...s, duration: s.duration - 1 })
              .filter(s => {
                  if (s.duration !== -1 && s.duration <= 0) {
                      expired.push(s.name);
                      member.aggroModifier -= (s.aggroModifier ?? 0); // subtract per-shield
                      return false;
                  }
                  return true;
              });
          if (expired.length > 0) {
              addLog(state, { type: 'event', message: `${member.name}'s shield(s) [${expired.join(', ')}] expired.` });
          }
          member.shield = member.activeShields.length > 0
              ? Math.max(...member.activeShields.map(s => s.value))
              : 0;
      }

      // ── Tick Zone durations (once per character turn) ─────────────────────
      {
          const expiredZones: string[] = [];
          state.activeZones = state.activeZones.map(z => ({ ...z, duration: z.duration - 1 }))
              .filter(z => { if (z.duration <= 0) { expiredZones.push(z.name); return false; } return true; });
          expiredZones.forEach(name => addLog(state, { type: 'info', message: `[ZONE] ${name} expired.` }));
      }

      // ── Tick Territory duration (once per character turn) ─────────────────
      if (state.activeTerritory) {
          state.activeTerritory = { ...state.activeTerritory, duration: state.activeTerritory.duration - 1 };
          if (state.activeTerritory.duration <= 0) {
              addLog(state, { type: 'info', message: `[TERRITORY] ${state.activeTerritory.name} expired.` });
              state.activeTerritory = null;
          }
      }

      if (!kit) {
        state.skillPoints++;
        actorInfo.nextAV += 10000 / spd;
        continue;
      }

      let actionType: 'basic' | 'skill' = 'basic';
      if (state.skillPoints > 0 && kit.name !== "Pela") { 
        actionType = 'skill';
        state.skillPoints--;
      } else {
        state.skillPoints++;
      }

      const target = state.enemies.find(e => e && e.hp > 0);
      if (!target) {
          checkEnemies(state);
          continue;
      }
      state.enemy = target;
      state.currentActionId = `${actorInfo.id}-${state.currentAV}-${actionType}`;

      let abilityLevel = actionType === 'basic' ? member.abilityLevels.basic : member.abilityLevels.skill;
      if (kit.special_modifiers.eidolon_level_boosts) {
          const boosts = kit.special_modifiers.eidolon_level_boosts(member.eidolon);
          if (actionType === 'basic' && boosts.basic) abilityLevel += boosts.basic;
          if (actionType === 'skill' && boosts.skill) abilityLevel += boosts.skill;
      }

      const rawAbility = actionType === 'basic' ? kit.abilities.basic : kit.abilities.skill;
      const slotName = actionType === 'basic' ? kit.slot_names.basic : kit.slot_names.skill;

      const aliveEnemies = state.enemies.filter((e): e is SimEnemy => e !== null && e.hp > 0);
      if (aliveEnemies.length === 0) {
          checkEnemies(state);
          continue;
      }

      const mainTarget = aliveEnemies[0];
      const mainTargetIdx = state.enemies.indexOf(mainTarget);

      addLog(state, { 
          type: 'action', 
          message: `Uses ${actionType.toUpperCase()}: ${slotName}`,
          actor: { id: member.characterId, name: member.name || member.characterId, type: 'ally', color: getActorColor(member.characterId, "", 'ally') },
          action: { type: actionType, name: slotName }
      });

      const scalingParts = rawAbility.default_multiplier ? { main: rawAbility } : rawAbility;
      
      Object.entries(scalingParts).forEach(([partName, partConfig]: [string, any]) => {
          const multiplier = resolveDatabaseScaling(member.databaseAbilities || [], slotName, abilityLevel, partConfig);
          const targetType = partConfig.targetType || 'SingleTarget';

          const action: Action = { 
            type: actionType, 
            multiplier, 
            stat_id: partConfig.stat_id, 
            distribution: partConfig.distribution,
            targetType,
            toughness_damage: partConfig.toughness_damage
          };

          const actionBuffs = { ...member.buffs };
          const actionMember = { ...member, buffs: actionBuffs };
          
          if (kit.hooks?.onBeforeAction) kit.hooks.onBeforeAction(state, actionMember, action, mainTarget);

          let targets: SimEnemy[] = [];
          if (targetType === 'SingleTarget' || (targetType === 'Blast' && partName === 'main')) {
              targets = [mainTarget];
          } else if (targetType === 'Blast' && partName === 'adjacent') {
              if (mainTargetIdx > 0 && state.enemies[mainTargetIdx - 1] && state.enemies[mainTargetIdx - 1]!.hp > 0) targets.push(state.enemies[mainTargetIdx - 1]!);
              if (mainTargetIdx < state.enemies.length - 1 && state.enemies[mainTargetIdx + 1] && state.enemies[mainTargetIdx + 1]!.hp > 0) targets.push(state.enemies[mainTargetIdx + 1]!);
          } else if (targetType === 'AoE') {
              targets = aliveEnemies;
          }

          targets.forEach(t => {
              const result = calculateHsrDamage({ 
                character: actionMember, 
                lightcone: member.lightcone, 
                enemy: t, 
                ability_multiplier: action.multiplier, 
                scaling_stat_id: action.stat_id 
              });
              
              const usedRes = (t.elemental_res && t.elemental_res[member.element] !== undefined)
                ? t.elemental_res[member.element]
                : t.resistance;

              addLog(state, { 
                  type: 'event', 
                  message: `Hit ${partName} on ${t.name} -> ${result.expected_dmg.toLocaleString()} DMG (HP: ${Math.max(0, Math.floor(t.hp - result.expected_dmg)).toLocaleString()}/${t.max_hp.toLocaleString()}) [RES: ${(usedRes * 100).toFixed(0)}% ${member.element}]`
              });

              state.totalDamage += result.expected_dmg;
              t.hp = Math.max(0, Math.floor(t.hp - result.expected_dmg));

              // Entanglement: each hit on the target adds 1 stack (max 5)
              const entangle = t.activeDebuffs["Entanglement"];
              if (entangle) {
                  const prev = entangle.stacks ?? 0;
                  entangle.stacks = Math.min(ENTANGLEMENT_MAX_STACKS, prev + 1);
                  if (entangle.stacks > prev) {
                      addLog(state, { type: 'event', message: `[Entanglement] ${t.name} hit  Estacks: ${entangle.stacks}/${ENTANGLEMENT_MAX_STACKS}` });
                  }
              }

              if (action.toughness_damage) {
                  const ignoreWeakness = (member.characterId === ACHERON_ID && (action.type === 'ultimate' || member.eidolon >= 6));
                  applyToughnessDamage(member, t, action.toughness_damage, state, ignoreWeakness);
              }
          });

          if (kit.hooks?.onAfterAction) kit.hooks.onAfterAction(state, member, action, mainTarget);
          
          if (action.inflictsDebuff) {
              targets.forEach(t => {
                team.forEach(m => {
                    if (HSR_CHARACTER_KITS[m.characterId]?.hooks?.onGlobalDebuff) HSR_CHARACTER_KITS[m.characterId].hooks!.onGlobalDebuff!(state, member, t);
                });
              });
          }
      });
      
      checkEnemies(state);
      state.checkAllies!();

      // Notify all characters to check follow-up conditions
      team.forEach(m => { if (!m.isDowned) { const k = HSR_CHARACTER_KITS[m.characterId]; if (k?.hooks?.onCheckFollowUp) k.hooks.onCheckFollowUp(state, m, 'after_action'); } });
      drainPendingActions();

      if (kit.special_modifiers.energy_type === "ENERGY") {
        const gain = actionType === 'skill' ? 30 : 20;
        state.stacks[member.characterId] = (state.stacks[member.characterId] || 0) + gain;
        addLog(state, { type: 'event', message: `${member.name} gains ${gain} Energy (Total: ${state.stacks[member.characterId]})` });
      }

      const energyReq = kit.special_modifiers.energy_cost || 120;
      const ultReady = kit.special_modifiers.energy_type === "STACKS" ? (state.stacks[actorInfo.id] >= 9) : (state.stacks[actorInfo.id] >= energyReq);
      if (ultReady && !state.isExtraTurn) {
          addLog(state, { type: 'info', message: '[ULTIMATE READY]' });
          const ultAction: Action = { type: 'ultimate', multiplier: 1, stat_id: ATK_ID, inflictsDebuff: true, is_ult_dmg: true };
          const ultBuffs = { ...member.buffs };
          const ultMember = { ...member, buffs: ultBuffs };
          
          const ultTarget = state.enemies.find(e => e && e.hp > 0);
          if (ultTarget) {
            state.currentActionId = `${actorInfo.id}-${state.currentAV}-ultimate`;
            if (kit.hooks?.onBeforeAction) kit.hooks.onBeforeAction(state, ultMember, ultAction, ultTarget);

            if (kit.hooks?.onUlt) {
                addLog(state, { 
                    type: 'action', 
                    message: `Uses ULTIMATE: ${kit.slot_names.ultimate}`,
                    actor: { id: member.characterId, name: member.name || member.characterId, type: 'ally', color: getActorColor(member.characterId, "", 'ally') },
                    action: { type: 'ultimate', name: kit.slot_names.ultimate }
                });
                const dmgBefore = state.totalDamage;
                (state as any).applyToughnessDamage = (t: SimEnemy, amt: number, ignore: boolean = false) => {
                    applyToughnessDamage(member, t, amt, state, ignore);
                };
                kit.hooks.onUlt(state, ultMember);
                const totalDmg = state.totalDamage - dmgBefore;
                const ultRes = (ultTarget.elemental_res && ultTarget.elemental_res[member.element] !== undefined)
                  ? ultTarget.elemental_res[member.element]
                  : ultTarget.resistance;
                addLog(state, { type: 'event', message: `Summary: ${totalDmg.toLocaleString()} total DMG [RES: ${(ultRes * 100).toFixed(0)}% ${member.element}]` });
            } else {
                addLog(state, { 
                    type: 'action', 
                    message: `Uses ULTIMATE: ${kit.slot_names.ultimate}`,
                    actor: { id: member.characterId, name: member.name || member.characterId, type: 'ally', color: getActorColor(member.characterId, "", 'ally') },
                    action: { type: 'ultimate', name: kit.slot_names.ultimate }
                });
                if (kit.special_modifiers.energy_type !== "STACKS") state.stacks[actorInfo.id] = 0;
                const scalingConfig = getPrimaryScalingConfig(kit.abilities.ultimate);
                const multiplier = resolveDatabaseScaling(member.databaseAbilities || [], kit.slot_names.ultimate, member.abilityLevels.ultimate, scalingConfig);
                const result = calculateHsrDamage({ 
                  character: ultMember, 
                  lightcone: member.lightcone, 
                  enemy: ultTarget, 
                  ability_multiplier: multiplier, 
                  scaling_stat_id: scalingConfig.stat_id 
                });
                state.totalDamage += result.expected_dmg;
                ultTarget.hp = Math.max(0, Math.floor(ultTarget.hp - result.expected_dmg));
                addLog(state, { type: 'event', message: `Hit on ${ultTarget.name} -> ${result.expected_dmg.toLocaleString()} DMG (HP: ${ultTarget.hp.toLocaleString()}/${ultTarget.max_hp.toLocaleString()})` });
                applyToughnessDamage(member, ultTarget, 20, state);
                team.forEach(m => {
                    if (HSR_CHARACTER_KITS[m.characterId]?.hooks?.onGlobalDebuff) HSR_CHARACTER_KITS[m.characterId].hooks!.onGlobalDebuff!(state, member, ultTarget);
                });
            }
            checkEnemies(state);
            // Follow-ups and extra turns triggered by the ult
            team.forEach(m => { if (!m.isDowned) { const k = HSR_CHARACTER_KITS[m.characterId]; if (k?.hooks?.onCheckFollowUp) k.hooks.onCheckFollowUp(state, m, 'after_ult'); } });
            drainPendingActions();
          }
      }

      actorInfo.nextAV += 10000 / (member.base_stats[CHAR_SPD_ID] || 100);
    } else if (actorInfo.id === 'aha') {
      // ── Aha's turn ──────────────────────────────────────────────────────
      handleAhaTurn();
      state.avQueue = state.avQueue.filter(a => a.id !== 'aha');
      continue;
    } else if (state.summons.some(s => s.id === actorInfo.id && s.instanceId === actorInfo.instanceId)) {
      // ── Summon / Memosprite turn ─────────────────────────────────────────
      const summonActor = state.summons.find(s => s.id === actorInfo.id && s.instanceId === actorInfo.instanceId)!;

      if (summonActor.isDowned || summonActor.hp <= 0) {
        state.dismissSummon(summonActor.instanceId);
        continue;
      }

      const summonMaster = team.find(m => m.characterId === summonActor.masterId);
      if (!summonMaster) {
        state.dismissSummon(summonActor.instanceId);
        continue;
      }

      // Tick summon status effects (same as ally, unless this is an extra turn)
      if (!state.isExtraTurn) {
        Object.entries(summonActor.activeDebuffs).forEach(([key, effect]) => {
          if (effect.duration > 0) effect.duration--;
          if (effect.duration <= 0) delete summonActor.activeDebuffs[key];
        });
      }

      addLog(state, {
        type: 'turn',
        message: `${summonActor.name} [${summonActor.kind === 'memosprite' ? 'MEMOSPRITE' : 'SUMMON'}]'s turn.`,
        actor: { id: summonActor.id, instanceId: summonActor.instanceId, name: summonActor.name, type: 'ally', color: getActorColor(summonMaster.characterId, "", 'ally') }
      });

      const masterKit = HSR_CHARACTER_KITS[summonMaster.characterId];
      if (masterKit?.hooks?.onSummonTurn) {
        masterKit.hooks.onSummonTurn(state, summonMaster, summonActor);
      } else {
        addLog(state, { type: 'event', message: `${summonActor.name} has no defined action.` });
      }

      checkEnemies(state);
      team.forEach(m => { if (!m.isDowned) { const k = HSR_CHARACTER_KITS[m.characterId]; if (k?.hooks?.onCheckFollowUp) k.hooks.onCheckFollowUp(state, m, 'after_action'); } });
      drainPendingActions();

      actorInfo.nextAV += 10000 / summonActor.spd;
    } else {
      const enemy = state.enemies.find(e => e && e.instanceId === actorInfo.instanceId && e.id === actorInfo.id);
      if (!enemy || enemy.hp <= 0) {
          // Remove only this specific actor  EcheckEnemies may have already removed it via filter.
          state.avQueue = state.avQueue.filter(a => !(a.id === actorInfo.id && a.instanceId === actorInfo.instanceId));
          continue;
      }

      // 1. Tick break DoTs BEFORE recovery (enemy still in is_broken state ↁEBrokenMult = 1.0).
      const isFrozen = tickBreakDoTs(enemy, state);
      checkEnemies(state);

      // 2. Freeze: turn skipped. tickBreakDoTs already set nextAV to currentAV + 50% gap.
      if (isFrozen) {
          addLog(state, { type: 'event', message: `${enemy.name} is Frozen  Eturn skipped.` });
          // Do NOT add another full AV here; tickBreakDoTs already advanced the schedule.
          continue;
      }

      // 3. If enemy died from DoT, checkEnemies already removed it from avQueue  Ejust continue.
      if (enemy.hp <= 0) { continue; }

      // 4. Recovery from Weakness Break (happens after DoTs tick).
      if (enemy.is_broken) {
          enemy.is_broken = false;
          enemy.toughness = enemy.max_toughness;
          addLog(state, { type: 'event', message: `${enemy.name} recovered from Weakness Break. Toughness restored to ${enemy.max_toughness}.` });
      }

      const spd = enemy.base_stats[ENEMY_SPD_ID] || 100;
      const atk = enemy.base_stats[ATK_ID] || 0;
      const resList = Object.entries(enemy.elemental_res)
        .map(([el, val]) => `${el}: ${(val * 100).toFixed(0)}%`)
        .join(", ");

      addLog(state, {
          type: 'turn',
          message: `${enemy.name}'s turn starts.`,
          actor: {
              id: enemy.id,
              instanceId: enemy.instanceId,
              name: enemy.name,
              type: 'enemy',
              color: getActorColor(enemy.id, enemy.instanceId, 'enemy')
          },
          subEntries: [
              `Stats: HP=${enemy.hp.toFixed(0)}/${enemy.max_hp.toFixed(0)}, ATK=${atk.toFixed(0)}, SPD=${spd.toFixed(1)}, RES=[${resList}], TGH=${enemy.toughness.toFixed(1)}/${enemy.max_toughness.toFixed(0)}`
          ]
      });

      state.currentActionId = `${enemy.id}-${state.currentAV}-enemy-turn`;
      const enemyKit = HSR_ENEMY_KITS[enemy.id];
      state.team.forEach(m => {
          const k = HSR_CHARACTER_KITS[m.characterId];
          if (k?.hooks?.onEnemyTurnStart) k.hooks.onEnemyTurnStart(state, m, enemy);
      });
      if (enemyKit?.hooks?.onTurnStart) enemyKit.hooks.onTurnStart(state, enemy);
      state.team.forEach(m => {
          const k = HSR_CHARACTER_KITS[m.characterId];
          if (k?.hooks?.onEnemyAction) k.hooks.onEnemyAction(state, m, enemy);
      });
      if (enemyKit?.hooks?.onAction) {
          enemyKit.hooks.onAction(state, enemy);
      } else {
          addLog(state, { type: 'event', message: `Action: Generic Attack` });
          // Generic attack: trigger onHit for the selected target (enables counters)
          const hitTarget = state.selectTarget(false);
          if (hitTarget) {
              const k = HSR_CHARACTER_KITS[hitTarget.characterId];
              if (k?.hooks?.onHit) k.hooks.onHit(state, hitTarget, enemy, 0);
          }
      }
      state.checkAllies!();
      // Commit the Mooncocoon once-per-battle charge now that the action is resolved.
      // All allies killed during onAction have already entered Mooncocoon; seal the trigger.
      if (state.mooncocoonPendingTrigger) {
          state.mooncocoonTriggered = true;
          state.mooncocoonPendingTrigger = false;
      }
      // Check for counter follow-ups triggered by the enemy attack
      drainPendingActions();
      actorInfo.nextAV += 10000 / spd;
    }
    state.team.forEach(m => checkMooncocoonRecovery(m, state));
  }

  if (!options.skipLogs) {
    addLog(state, { type: 'header', message: 'SIMULATION ENDED' });
    addLog(state, { type: 'info', message: `Total Team Damage: ${state.totalDamage.toLocaleString()}` });
  }

  return {
    totalDamage: state.totalDamage,
    cyclesTaken: Math.ceil((state.currentAV - 150) / 100) + 1,
    logs: state.logs,
    isDefeated,
    finalTeam: state.team
  };
}
