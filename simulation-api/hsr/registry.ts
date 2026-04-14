import type { CharacterKit, EnemyKit } from "./types.js";
import { Acheron } from "./characters/acheron.js";
import { SilverWolf } from "./characters/silver-wolf.js";
import { Pela } from "./characters/pela.js";
import { Jiaoqiu } from "./characters/jiaoqiu.js";
import { Anaxa } from "./characters/anaxa.js";
import { Archer } from "./characters/archer.js";
import { Aglaea } from "./characters/aglaea.js";
import { Argenti } from "./characters/argenti.js";
import { Arlan } from "./characters/arlan.js";
import { Ashveil } from "./characters/ashveil.js";
import { Asta } from "./characters/asta.js";
import { Aventurine } from "./characters/aventurine.js";
import { Bailu } from "./characters/bailu.js";
import { Antibaryon } from "./enemies/antibaryon.js";
import { Baryon } from "./enemies/baryon.js";

export const HSR_CHARACTER_KITS: Record<string, CharacterKit> = {
  [Acheron.id]: Acheron,
  [SilverWolf.id]: SilverWolf,
  [Pela.id]: Pela,
  [Jiaoqiu.id]: Jiaoqiu,
  [Anaxa.id]: Anaxa,
  [Archer.id]: Archer,
  [Aglaea.id]: Aglaea,
  [Argenti.id]: Argenti,
  [Arlan.id]: Arlan,
  [Ashveil.id]: Ashveil,
  [Asta.id]: Asta,
  [Aventurine.id]: Aventurine,
  [Bailu.id]: Bailu,
};

export const HSR_ENEMY_KITS: Record<string, EnemyKit> = {
  [Antibaryon.id]: Antibaryon,
  [Baryon.id]: Baryon,
};
