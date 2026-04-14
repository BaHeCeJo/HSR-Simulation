-- Migration: Add Ability Scaling Base Stat Support

-- 1. Add scaling_stat_id to entity_ability_scaling
ALTER TABLE public.entity_ability_scaling 
ADD COLUMN IF NOT EXISTS scaling_stat_id uuid REFERENCES public.section_stats(id) ON DELETE SET NULL;

-- 2. Update indices if necessary (optional)
CREATE INDEX IF NOT EXISTS idx_entity_ability_scaling_stat ON public.entity_ability_scaling(scaling_stat_id);
