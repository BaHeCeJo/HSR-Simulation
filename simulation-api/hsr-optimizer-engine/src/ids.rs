#![allow(dead_code)]
// ─── Character stat UUIDs (from HSR_ID_MAPPING.md) ──────────────────────────
pub const CHAR_HP_ID:       &str = "7383172e-f828-4298-a8cf-887d50ff4a28";
pub const CHAR_ATK_ID:      &str = "c987f652-6a0b-487f-9e4b-af2c9b51c6aa";
pub const CHAR_DEF_ID:      &str = "73868117-3df2-470d-945a-e389f9f04200";
pub const CHAR_SPD_ID:      &str = "3e4b082d-7943-440d-ae2c-8d31b0a370be";
pub const CHAR_CR_ID:       &str = "a62e3a38-743a-41f8-8523-aec4ef998c84";
pub const CHAR_CD_ID:       &str = "a93e523a-7852-4580-b2ef-03467e214bcd";
pub const CHAR_BE_ID:       &str = "268dd0de-bada-4dd1-ae1a-e1019290dab7";
pub const CHAR_ERR_ID:         &str = "f01bc2c1-9a3e-4e54-98b0-7e12fc3d4a5b";
pub const CHAR_EHR_ID:         &str = "b8a7c6d5-e4f3-4210-9b8a-7c6d5e4f3210";
pub const CHAR_EFFECT_RES_ID:  &str = "c5d6e7f8-a9b0-c1d2-e3f4-a5b6c7d8e9f0";

// ─── Lightcone stat UUIDs ────────────────────────────────────────────────────
pub const LC_HP_ID:         &str = "1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d";
pub const LC_ATK_ID:        &str = "8e5af9db-3079-49ef-90c3-747b4ea00025";
pub const LC_DEF_ID:        &str = "2b3c4d5e-6f7a-8b9c-0d1e-2f3a4b5c6d7e";

// ─── Enemy stat UUIDs ────────────────────────────────────────────────────────
pub const ENEMY_HP_ID:         &str = "dab1d58a-5e35-470a-a2d4-1bdddf3019a0";
pub const ENEMY_ATK_ID:        &str = "7761c316-9c6b-4610-aa72-afcb80aeb1e9";
pub const ENEMY_DEF_ID:        &str = "f2a3b4c5-d6e7-f8a9-b0c1-d2e3f4a5b6c7";
pub const ENEMY_SPD_ID:        &str = "b0bfd27b-0a5f-4329-a280-dc1c998446cb";
pub const ENEMY_TOUGHNESS_ID:  &str = "50ff424d-9428-46e2-8f3e-8968dacbb6bd";
pub const ENEMY_EHR_ID:        &str = "a3b4c5d6-e7f8-a9b0-c1d2-e3f4a5b6c7d8";
pub const ENEMY_EFFECT_RES_ID: &str = "b4c5d6e7-f8a9-b0c1-d2e3-f4a5b6c7d8e9";

// ─── Element type → UUID mapping ─────────────────────────────────────────────
pub const ELEM_PHYSICAL_ID:  &str = "441500bc-47dc-452f-9f1f-f0aaa142ce62";
pub const ELEM_FIRE_ID:      &str = "2c50d8d8-3d62-4e5d-8221-68f28d8cdddb";
pub const ELEM_ICE_ID:       &str = "cc934b70-7aca-46dc-beb2-0aafc221e2ec";
pub const ELEM_LIGHTNING_ID: &str = "3de09fc5-7cb1-412f-aac0-0ebb0ba905e8";
pub const ELEM_WIND_ID:      &str = "4c775af5-281e-4bbb-8ca2-b2e3f20f3c18";
pub const ELEM_QUANTUM_ID:   &str = "9deee2d8-f7bf-41b7-829e-2485837784df";
pub const ELEM_IMAGINARY_ID: &str = "176151ff-8d54-4b1b-98fb-03ef410d7371";

// ─── Character kit IDs ───────────────────────────────────────────────────────
pub const ACHERON_ID:     &str = "f06222e4-d23d-4ac2-86ff-3a6cc389b812";
pub const BLACK_SWAN_ID:  &str = "a0b1c2d3-e4f5-6789-a0b1-c2d3e4f56789";
pub const JIAOQIU_ID:     &str = "f06222e4-d23d-4ac2-86ff-3a6cc389b813";
pub const PELA_ID:        &str = "a8a4d435-bcd4-4105-83ab-72650f296844";
pub const SILVER_WOLF_ID: &str = "2f2432f3-4736-4210-870a-c48d0c6bc3ee";
pub const AGLAEA_ID:      &str = "e1e8adcb-ba8f-4c38-adec-cc5c9bfe09e1";
pub const GARMENTMAKER_ID:&str = "b9d5f7a3-c4e6-4901-9bcd-f01234567890";
pub const ANAXA_ID:       &str = "7e8f9a0b-1c2d-3e4f-5a6b-7c8d9e0f1a2b";
pub const ARCHER_ID:      &str = "e1f2a3b4-c5d6-e7f8-a9b0-c1d2e3f4a5b6";
pub const ARGENTI_ID:     &str = "e1a2b3c4-d5e6-4f7a-8b9c-0d1e2f3a4b5c";
pub const ARLAN_ID:       &str = "d2c3b4a5-e6f7-4a8b-9c0d-1e2f3a4b5c6d";
pub const ASHVEIL_ID:     &str = "f1e2d3c4-b5a6-4789-a0bc-de1f2a3b4c5d";
pub const ASTA_ID:        &str = "a8b9c0d1-e2f3-4567-89ab-cdef01234567";
pub const AVENTURINE_ID:  &str = "c2d3e4f5-a6b7-4890-c1d2-e3f4a5b6c7d8";
pub const BAILU_ID:       &str = "d3e4f5a6-b7c8-4901-d2e3-f4a5b6c7d8e9";
pub const BLADE_ID:       &str = "e4f5a6b7-c8d9-4e0f-a1b2-c3d4e5f6a7b8";
pub const BOOTHILL_ID:    &str = "f5a6b7c8-d9e0-4f1a-b2c3-d4e5f6a7b8c9";
pub const BRONYA_ID:      &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
pub const CASTORICE_ID:   &str = "b2c3d4e5-f6a7-8901-bcde-f23456789012";
pub const CERYDRA_ID:     &str = "d4e5f6a7-b8c9-0123-def0-456789012345";
pub const CLARA_ID:       &str = "a6b7c8d9-e0f1-2345-a6b7-c8d9e0f12345";
pub const DAN_HENG_ID:    &str = "9f8e7d6c-5b4a-3928-9f8e-7d6c5b4a3928";
pub const DAN_HENG_IL_ID: &str = "d1e2f3a4-b5c6-7890-d1e2-f3a4b5c67890";
pub const DAN_HENG_PT_ID: &str = "e3f4a5b6-c7d8-9012-e3f4-a5b6c7d89012";
pub const DR_RATIO_ID:    &str = "a7b8c9d0-e1f2-3456-a7b8-c9d0e1f23456";
pub const FEIXIAO_ID:     &str = "b8c9d0e1-f2a3-4567-b8c9-d0e1f2a34567";
pub const FIREFLY_ID:     &str = "c6d7e8f9-a0b1-2345-c6d7-e8f9a0b12345";
pub const FU_XUAN_ID:     &str = "d7e8f9a0-b1c2-3456-d7e8-f9a0b1c23456";
pub const GALLAGHER_ID:   &str = "e8f9a0b1-c2d3-4567-e8f9-a0b1c2d34567";
pub const GEPARD_ID:      &str = "f0a1b2c3-d4e5-6789-f0a1-b2c3d4e56789";
pub const GUINAIFEN_ID:   &str = "a1b2c3d4-e5f6-7890-a1b2-c3d4e5f67890";
pub const SOULDRAGON_ID:  &str = "f4a5b6c7-d8e9-0123-f4a5-b6c7d8e90123";
pub const CIPHER_ID:      &str = "e5f6a7b8-c9d0-1234-ef01-567890123456";
pub const NETHERWING_ID:  &str = "c3d4e5f6-a7b8-9012-cdef-345678901234";

// ─── Lightcone passive IDs ───────────────────────────────────────────────────
pub const LC_DREAM_SCENTED_WHEAT_ID:    &str = "df8260b3-e438-4f9e-b942-2e746bb256f4";
pub const LC_ALONG_THE_PASSING_SHORE_ID: &str = "8a6a5884-8ec4-4d7b-8aa1-7444844c5734";

// ─── Enemy kit IDs ───────────────────────────────────────────────────────────
pub const ANTIBARYON_ID:  &str = "50cf7b6b-c373-4ee8-ace8-13bf101e0f0f";
pub const BARYON_ID:      &str = "962d69dc-fbff-47b4-bfa0-cd4b0358d80b";
