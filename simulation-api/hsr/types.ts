/**
 * Shared types for HSR combat simulation components.
 */

export interface AbilityHitDistribution {
  hits: number[]; // e.g., [0.3, 0.3, 0.4] for 30%/30%/40% distribution
}

export type AbilityTargetType = 
  | 'SingleTarget' 
  | 'Blast' 
  | 'AoE' 
  | 'Bounce' 
  | 'Enhance' 
  | 'Support' 
  | 'Defense' 
  | 'Restore' 
  | 'Summon';

/**
 * AbilityScaling defines how an ability scales.
 */
export interface AbilityScaling {
  ability_id?: string;
  attribute_id?: string;
  attribute_index?: number;
  default_multiplier: number;
  stat_id: string;
  distribution?: AbilityHitDistribution;
  targetType?: AbilityTargetType;
  toughness_damage?: number; // Base toughness reduction (e.g. 10 for basic)
}

export type ActionType = 'basic' | 'skill' | 'ultimate' | 'follow_up' | 'talent_proc';

export interface Action {
  type: ActionType;
  multiplier: number;
  stat_id: string;
  is_ult_dmg?: boolean;
  distribution?: AbilityHitDistribution;
  inflictsDebuff?: boolean;
  targetType?: AbilityTargetType;
  toughness_damage?: number;
}

export interface StatusEffect {
  duration: number;
  value?: number;
  stat?: string; // e.g., 'ATK', 'Crit DMG', 'DEF'
  effects?: { value: number; stat: string }[]; // Support for multiple stat changes in one effect
}

export interface SimEnemy {
  id: string; // The kit ID
  instanceId: string; // To distinguish between multiples of the same enemy
  name: string;
  level: number;
  hp: number;
  max_hp: number;
  toughness: number;
  max_toughness: number;
  weaknesses: string[]; // e.g. ["Lightning", "Fire"]
  resistance: number; // Base resistance (fallback)
  elemental_res: Record<string, number>; // e.g., { "Lightning": 0.2, "Quantum": 0.0 }
  is_broken: boolean;
  vulnerability: number;
  dmg_reduction: number;
  weaken: number;
  debuffCount: number; 
  activeDebuffs: Record<string, StatusEffect>; // Status on enemy
  activeBuffs: Record<string, StatusEffect>; // Status on enemy
  base_stats: Record<string, number>; // Using UUIDs from HSR_ID_MAPPING.md
}

export interface Wave {
  initialEnemies: (SimEnemy | null)[]; // Max 5, null for empty slot
  enemyPool: SimEnemy[]; // Enemies waiting to be summoned
}

export interface TeamMember {
  characterId: string;
  name?: string;
  element: string;
  path?: string;
  level: number;
  eidolon: number;
  hp: number;
  max_hp: number;
  shield: number;
  isDowned?: boolean; // True if the character is defeated
  loggedDowned?: boolean; // To avoid duplicate logs
  mooncocoon?: boolean; // Castorice survival state
  loggedMooncocoon?: boolean; // To avoid duplicate logs
  mooncocoon_expiry?: boolean; // If true, the character will be downed at the start of their next turn if not recovered
  toughness: number;
  max_toughness: number;
  is_broken: boolean;
  abilityLevels: {
    basic: number;
    skill: number;
    ultimate: number;
    talent: number;
  };
  databaseAbilities?: any[]; 
  base_stats: Record<string, number>;
  buffs: {
    atk_percent: number;
    crit_rate: number;
    crit_dmg: number;
    dmg_boost: number;
    def_ignore: number;
    extra_multiplier: number;
    extra_dmg: number;
    res_pen: number;
  };
  activeBuffs: Record<string, StatusEffect>;
  activeDebuffs: Record<string, StatusEffect>;
  lightcone: {
    base_stats: Record<string, number>;
    scaling: number;
  };
}

export interface LogEntry {
  av: number;
  type: 'action' | 'turn' | 'event' | 'header' | 'wave' | 'info' | 'defeat' | 'victory';
  actor?: {
    id: string;
    instanceId?: string;
    name: string;
    type: 'ally' | 'enemy';
    color?: string;
  };
  action?: {
    type: string;
    name: string;
  };
  message: string;
  subEntries?: string[];
}

export interface SimState {
  team: TeamMember[];
  enemies: (SimEnemy | null)[];
  enemy: SimEnemy;
  enemyPool: SimEnemy[];
  waves: Wave[];
  currentWaveIndex: number;
  currentAV: number;
  maxAV: number;
  skillPoints: number;
  totalDamage: number;
  logs: LogEntry[];
  
  addLog: (entry: Omit<LogEntry, 'av'>) => void;
  
  // Character specific state
  stacks: Record<string, number>;
  buffDurations: Record<string, Record<string, StatusEffect>>;
  turnCounters: Record<string, number>;
  avQueue: { id: string; instanceId: string; nextAV: number }[];
  
  nihilityCount: number;
  currentActionId?: string;
  hasCastoricePassive?: boolean; // Whether the user has Castorice or she is in team
  mooncocoonTriggered?: boolean; // Only triggers once per battle

  // Summons / Memosprites (e.g. Garmentmaker)  Ekeyed by characterId
  summons?: Record<string, TeamMember>;
  // Set by kit hooks to override the standard nextAV increment after a turn
  immediateAction?: Record<string, boolean>;

  applyDamageToAlly?: (member: TeamMember, damage: number, toughnessDamage?: number) => void;
  checkEnemies?: () => void;
  checkAllies?: () => void;
}

export interface SimReport {
  totalDamage: number;
  cyclesTaken: number;
  logs: LogEntry[];
  isDefeated: boolean;
}

/**
 * CharacterHooks allow characters to inject logic into the combat loop
 * without modifying the simulator itself.
 */
export interface CharacterHooks {
  onBattleStart?: (state: SimState, member: TeamMember) => void;
  onTurnStart?: (state: SimState, member: TeamMember) => void;
  onBeforeAction?: (state: SimState, member: TeamMember, action: Action, target?: SimEnemy) => void;
  onAfterAction?: (state: SimState, member: TeamMember, action: Action, target?: SimEnemy) => void;
  onUlt?: (state: SimState, member: TeamMember) => void;
  // Triggered when ANY team member inflicts a debuff (crucial for Acheron)
  onGlobalDebuff?: (state: SimState, source: TeamMember, target: SimEnemy) => void;
  // New hooks for Jiaoqiu and others
  onEnemyTurnStart?: (state: SimState, member: TeamMember, enemy: SimEnemy) => void;
  onEnemyAction?: (state: SimState, member: TeamMember, enemy: SimEnemy) => void;
  onEnemyDefeated?: (state: SimState, member: TeamMember, enemy: SimEnemy) => void;
  // Triggered after any ALLY's basic/skill/ultimate  Elets passives (e.g. Ashveil) react to teammate actions
  onAllyAction?: (state: SimState, member: TeamMember, source: TeamMember, actionType: string, target?: SimEnemy) => void;
}

export interface CharacterKit {
  id: string; // Character UUID
  name: string;
  path: string;
  element: string;
  
  // Mapping standard slots to database definition names (English)
  slot_names: {
    basic: string;
    skill: string;
    ultimate: string;
    talent?: string;
  };

  abilities: {
    basic: AbilityScaling | Record<string, AbilityScaling>;
    skill: AbilityScaling | Record<string, AbilityScaling>;
    ultimate: AbilityScaling | Record<string, AbilityScaling>;
    talent?: AbilityScaling | Record<string, AbilityScaling>;
  };

  hooks?: CharacterHooks;

  // Special mechanics (Legacy/Standardized modifiers)
  special_modifiers: {
    energy_type: "ENERGY" | "STACKS" | "NONE";
    energy_cost?: number;
    // Multipliers and boosts that are constant or easily calculated
    multiplicative_dmg_multiplier?: (teamState: any) => number;
    eidolon_level_boosts?: (eidolon: number) => Record<string, number>;
    stat_boosts?: (stats: any) => Record<string, number>;
  };
}

export interface LightConeKit {
  id: string;
  name: string;
  // TODO: Add lightcone-specific mechanics
}

export interface EnemyKit {
  id: string;
  name: string;
  hooks?: {
    onBattleStart?: (state: SimState, enemy: SimEnemy) => void;
    onTurnStart?: (state: SimState, enemy: SimEnemy) => void;
    onAction?: (state: SimState, enemy: SimEnemy) => void;
  };
}

export interface RelicSetKit {
  id: string;
  name: string;
  // TODO: Add relic-specific mechanics
}
