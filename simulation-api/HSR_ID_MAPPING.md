# HSR Database ID Mapping (Optimizer Skeleton)

This file contains the definitive UUIDs for the Honkai: Star Rail Optimizer. The Rust/WASM engine uses these IDs to perform high-performance calculations without string lookups.

## 1. Core IDs
- **Game ID:** `dbd417da-9908-44fe-b854-78f94351f025`

### Section IDs
- **Characters:** `b4a3d27a-09c1-44b5-8507-c6a4eb0b0b6b`
- **Lightcones:** `1fe8b8e8-c502-4f89-a83d-3ab813e3dac9`
- **Relics:** `24b5a50a-d003-4c5e-8bf0-acd8d95a6b18`
- **Planar Ornaments:** `a1871d06-3c36-43fa-8426-a651ce0f2222`
- **Enemies:** `cf6e5077-c2c5-44dd-9bc4-66563a954350`

## 2. Character Stats (section_stats)
- **HP:** `7383172e-f828-4298-a8cf-887d50ff4a28`
- **DEF:** `73868117-3df2-470d-945a-e389f9f04200`
- **ATK:** `c987f652-6a0b-487f-9e4b-af2c9b51c6aa`
- **SPD:** `3e4b082d-7943-440d-ae2c-8d31b0a370be`
- **Crit Rate:** `a62e3a38-743a-41f8-8523-aec4ef998c84`
- **Crit DMG:** `a93e523a-7852-4580-b2ef-03467e214bcd`
- **Break Effect (BE):** `268dd0de-bada-4dd1-ae1a-e1019290dab7`
- **Energy Regen Rate (ERR):** `5dcab875-0b56-4696-966e-49c1e485226b`
- **Effect Hit Rate (EHR):** `764bb8fd-ad42-4bd9-8332-631187154d77`
- **Elemental DMG Boost:** `5169b8ca-b8c5-4bfc-9570-7f194789dfd7`

## 3. Lightcone Stats (section_stats)
- **HP:** `2cd56050-3fb9-4bb5-a226-17aad0d34e53`
- **DEF:** `52566b38-915c-4220-ab0e-61438225704b`
- **ATK:** `8e5af9db-3079-49ef-90c3-747b4ea00025`

## 4. Enemy Stats (section_stats)
### Core attributes
- **HP:** `dab1d58a-5e35-470a-a2d4-1bdddf3019a0`
- **ATK:** `7761c316-9c6b-4610-aa72-afcb80aeb1e9`
- **DEF:** `7b58e059-a7ec-4535-a685-8961e5bc518d`
- **SPD:** `b0bfd27b-0a5f-4329-a280-dc1c998446cb`
- **EHR:** `3acbd4a1-95d3-490b-a60a-33e4afbedabd`
- **Effect RES:** `5c19ad45-af96-4f7d-9f3d-9886a91ed4bb`
- **Toughness:** `50ff424d-9428-46e2-8f3e-8968dacbb6bd`

### Elemental RES
- **Physical RES:** `441500bc-47dc-452f-9f1f-f0aaa142ce62`
- **Fire RES:** `2c50d8d8-3d62-4e5d-8221-68f28d8cdddb`
- **Ice RES:** `cc934b70-7aca-46dc-beb2-0aafc221e2ec`
- **Lightning RES:** `3de09fc5-7cb1-412f-aac0-0ebb0ba905e8`
- **Wind RES:** `4c775af5-281e-4bbb-8ca2-b2e3f20f3c18`
- **Quantum RES:** `9deee2d8-f7bf-41b7-829e-2485837784df`
- **Imaginary RES:** `176151ff-8d54-4b1b-98fb-03ef410d7371`

### Crowd Control (CC) RES
- **Bleed RES:** `9ecf258c-8f10-4941-a01e-2719bf5ac2f8`
- **Burn RES:** `9f549ee8-82b7-4163-a76c-7a41dc778ac5`
- **Frozen RES:** `17f87469-ffaf-4544-a019-9c76099d02cd`
- **Shock RES:** `fe4654fa-d39e-4b36-adbd-e003515685b8`
- **Wind Shear RES:** `1aa2e98c-979e-47fa-9af3-c0b314715562`
- **Entanglement RES:** `95593691-d658-4f18-b7f4-41c5102734af`
- **Imprisonment RES:** `7154f1fb-f39f-401f-9cee-c14f3a7ceca8`
- **Control Effects RES:** `3d35e39e-52e9-41ce-8723-35497e43ff03`
