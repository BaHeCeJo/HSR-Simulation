# Character Kit Definitions

This directory contains the "source of truth" markdown files for Honkai: Star Rail character kits.

## Purpose
1. **Reference**: Provide a human-readable (and AI-readable) definition of a character's abilities, eidolons, and special mechanics.
2. **Implementation Guide**: Serve as the blueprint for creating or updating the corresponding `.ts` implementation in `lib/hsr/characters/`.
3. **Verification**: Allow users to verify if the implemented logic matches the intended kit.

## ⚠️ Implementation Guidelines (FOR AI)
When implementing or updating a character from these files:
- **Scalings**: Look for values wrapped in curly braces like `{100%}` or `{45%}`. These are the primary multipliers and base chances for the current level (e.g. Lv. 6 for Basic, Lv. 10 for others).
- **Traces/Eidolons**: Ensure every Major Trace (A2, A4, A6) and Eidolon (E1-E6) is accounted for in the `.ts` hooks.
- **UUIDs**: Use the official `entity_id` provided by the user or found in the database.

## Workflow
When implementing a new character:
1. Create a `{character-name}.md` file here.
2. Define the Overview, Abilities, Eidolons, and expected Simulation Logic using the `{}` notation for all numerical scalings.
3. Ask the AI to implement the kit in `lib/hsr/characters/{character-name}.ts` based on the `.md` file.
