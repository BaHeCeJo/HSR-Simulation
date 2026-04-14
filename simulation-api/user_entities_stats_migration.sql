-- Add level and phase_index to user_entities
ALTER TABLE user_entities ADD COLUMN level integer DEFAULT 1;
ALTER TABLE user_entities ADD COLUMN phase_index integer DEFAULT 0;

-- Optional: Update RPC if needed. 
-- Assuming add_entity_to_user handles the initial insert.
-- It might look something like this:
-- CREATE OR REPLACE FUNCTION add_entity_to_user(p_user_id uuid, p_entity_id uuid)
-- RETURNS void AS $$
-- BEGIN
--   INSERT INTO user_entities (user_id, entity_id, dupes, level, phase_index)
--   VALUES (p_user_id, p_entity_id, 0, 1, 0);
-- END;
-- $$ LANGUAGE plpgsql;
