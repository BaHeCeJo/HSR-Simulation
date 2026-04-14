-- Migration: Add Ability Scaling Support

-- 1. Add max_level to section_ability_definitions
ALTER TABLE public.section_ability_definitions 
ADD COLUMN IF NOT EXISTS max_level integer NOT NULL DEFAULT 1;

-- 2. Create entity_ability_scaling table
CREATE TABLE IF NOT EXISTS public.entity_ability_scaling (
  id uuid NOT NULL DEFAULT gen_random_uuid(),
  ability_id uuid NOT NULL,
  attribute_index integer NOT NULL, -- 0 for Attribute 1, 1 for Attribute 2...
  level integer NOT NULL,
  value numeric NOT NULL DEFAULT 0,
  value_type text NOT NULL DEFAULT 'percent', -- 'percent' or 'flat'
  created_at timestamp with time zone NOT NULL DEFAULT now(),

  CONSTRAINT entity_ability_scaling_pkey PRIMARY KEY (id),
  CONSTRAINT entity_ability_scaling_ability_id_fkey 
    FOREIGN KEY (ability_id) 
    REFERENCES public.entity_abilities(id) 
    ON DELETE CASCADE,
  -- Ensure we don't have duplicate level/attribute entries for the same ability
  CONSTRAINT entity_ability_scaling_unique_entry 
    UNIQUE (ability_id, attribute_index, level)
);

-- 3. Index for performance
CREATE INDEX IF NOT EXISTS idx_entity_ability_scaling_ability ON public.entity_ability_scaling(ability_id);

-- 4. Enable RLS
ALTER TABLE public.entity_ability_scaling ENABLE ROW LEVEL SECURITY;

-- 5. Policies
-- Public read access
CREATE POLICY "Allow public read access for ability scaling"
  ON public.entity_ability_scaling
  FOR SELECT
  TO public
  USING (true);

-- Admin full access (assuming 'admin' role exists in profiles)
CREATE POLICY "Allow admin full access for ability scaling"
  ON public.entity_ability_scaling
  FOR ALL
  TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM public.profiles
      WHERE id = auth.uid() AND role = 'admin'
    )
  );
