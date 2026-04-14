/**
 * @enemy Antibaryon
 * @id 50cf7b6b-c373-4ee8-ace8-13bf101e0f0f
 * @ability Obliterate: Deals minor Imaginary DMG (250% ATK) to a single target.
 */

import type { EnemyKit, SimState, SimEnemy } from "../types.js";
import { HSR_CHARACTER_KITS } from "../registry.js";

export const Antibaryon: EnemyKit = {
  id: "50cf7b6b-c373-4ee8-ace8-13bf101e0f0f",
  name: "Antibaryon",
  hooks: {
    onAction: (state: SimState, enemy: SimEnemy) => {
      const ENEMY_ATK_ID = '7761c316-9c6b-4610-aa72-afcb80aeb1e9';
      const CHAR_DEF_ID = '73868117-3df2-470d-945a-e389f9f04200';

      // Enemy logic: Select target and perform OBLITERATE
      const aliveMembers = state.team.filter(m => m.hp > 0);
      if (aliveMembers.length === 0) return;

      const targetIndex = Math.floor(Math.random() * aliveMembers.length);
      const target = aliveMembers[targetIndex];
      
      // Calculate Damage: (Enemy ATK * Multiplier) * DEF Multiplier
      const enemyAtk = enemy.base_stats[ENEMY_ATK_ID] || 500;
      const multiplier = 2.5; // Obliterate: 250% ATK
      const baseDmg = enemyAtk * multiplier;

      // DEF Multiplier = (Attacker_Lv * 10 + 200) / (Defender_DEF + Attacker_Lv * 10 + 200)
      const defenderDef = target.base_stats[CHAR_DEF_ID] || 600;
      const defMult = (enemy.level * 10 + 200) / (defenderDef + enemy.level * 10 + 200);
      
      const finalDmg = Math.floor(baseDmg * defMult);
      
      const enemyName = `${Antibaryon.name} (${enemy.instanceId})`;
      const charName = HSR_CHARACTER_KITS[target.characterId]?.name || target.characterId;
      
      // Use state-provided helper for downstream logic (Mooncocoon)
      const hpBefore = target.hp;
      const toughnessDamage = 10;
      if ((state as any).applyDamageToAlly) {
          (state as any).applyDamageToAlly(target, finalDmg, state, toughnessDamage);
      } else {
          target.hp = Math.max(0, target.hp - finalDmg);
      }

      state.addLog({ 
          type: 'event', 
          message: `Action: OBLITERATE on ${charName} -> ${finalDmg} DMG (HP: ${target.hp}/${target.max_hp}, TGH: ${target.toughness.toFixed(1)}/${target.max_toughness}) by ${enemyName}` 
      });
    }
  }
};
