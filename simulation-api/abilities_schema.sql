-- =========================================
-- 1. Section Ability Templates
-- =========================================
CREATE TABLE IF NOT EXISTS public.section_ability_templates (
  id uuid NOT NULL DEFAULT gen_random_uuid(),
  section_id uuid NOT NULL,
  name jsonb NOT NULL, -- LocalizedString (e.g. "Standard Kit")
  is_default boolean NOT NULL DEFAULT false,
  order_index integer NOT NULL DEFAULT 0,
  created_at timestamp with time zone NOT NULL DEFAULT now(),

  CONSTRAINT section_ability_templates_pkey PRIMARY KEY (id),
  CONSTRAINT section_ability_templates_section_id_fkey 
    FOREIGN KEY (section_id) 
    REFERENCES public.game_sections(id) 
    ON DELETE CASCADE
);

-- =========================================
-- 2. Section Ability Definitions (The Slots)
-- =========================================
CREATE TABLE IF NOT EXISTS public.section_ability_definitions (
  id uuid NOT NULL DEFAULT gen_random_uuid(),
  template_id uuid NOT NULL,
  name jsonb NOT NULL, -- LocalizedString (e.g. "Skill", "Ultimate")
  order_index integer NOT NULL DEFAULT 0,
  created_at timestamp with time zone NOT NULL DEFAULT now(),

  CONSTRAINT section_ability_definitions_pkey PRIMARY KEY (id),
  CONSTRAINT section_ability_definitions_template_id_fkey 
    FOREIGN KEY (template_id) 
    REFERENCES public.section_ability_templates(id) 
    ON DELETE CASCADE
);

-- =========================================
-- 3. Entity Abilities (The Content)
-- =========================================
CREATE TABLE IF NOT EXISTS public.entity_abilities (
  id uuid NOT NULL DEFAULT gen_random_uuid(),
  entity_id uuid NOT NULL,
  definition_id uuid NOT NULL,
  name jsonb NOT NULL, -- LocalizedString (The actual name of the skill)
  description jsonb NOT NULL DEFAULT '{}'::jsonb, -- LocalizedString
  icon_path text,
  created_at timestamp with time zone NOT NULL DEFAULT now(),

  CONSTRAINT entity_abilities_pkey PRIMARY KEY (id),
  CONSTRAINT entity_abilities_entity_id_fkey 
    FOREIGN KEY (entity_id) 
    REFERENCES public.section_entities(id) 
    ON DELETE CASCADE,
  CONSTRAINT entity_abilities_definition_id_fkey 
    FOREIGN KEY (definition_id) 
    REFERENCES public.section_ability_definitions(id) 
    ON DELETE CASCADE,
  CONSTRAINT entity_abilities_unique_slot 
    UNIQUE (entity_id, definition_id)
);

-- =========================================
-- 4. Entity Ability Forms (Alternate versions)
-- =========================================
CREATE TABLE IF NOT EXISTS public.entity_ability_forms (
  id uuid NOT NULL DEFAULT gen_random_uuid(),
  ability_id uuid NOT NULL,
  name jsonb NOT NULL, -- LocalizedString (e.g. "Enhanced Basic")
  description jsonb NOT NULL DEFAULT '{}'::jsonb, -- LocalizedString
  icon_path text,
  order_index integer NOT NULL DEFAULT 0,
  created_at timestamp with time zone NOT NULL DEFAULT now(),

  CONSTRAINT entity_ability_forms_pkey PRIMARY KEY (id),
  CONSTRAINT entity_ability_forms_ability_id_fkey 
    FOREIGN KEY (ability_id) 
    REFERENCES public.entity_abilities(id) 
    ON DELETE CASCADE
);

-- =========================================
-- 5. Indexes
-- =========================================
CREATE INDEX IF NOT EXISTS idx_ability_templates_section ON public.section_ability_templates(section_id);
CREATE INDEX IF NOT EXISTS idx_ability_definitions_template ON public.section_ability_definitions(template_id);
CREATE INDEX IF NOT EXISTS idx_entity_abilities_entity ON public.entity_abilities(entity_id);
CREATE INDEX IF NOT EXISTS idx_ability_forms_ability ON public.entity_ability_forms(ability_id);
