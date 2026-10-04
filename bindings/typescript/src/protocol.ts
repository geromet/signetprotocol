// Signet/1, the neutral protocol: JSON, one message per line, over TCP (port 7777).
// Mirror of `signet_sdk::protocolo` (Rust). The wire vocabulary is in Spanish;
// each field is documented in English. Optional fields may be missing in older
// servers or clients.

export const PORT = 7777;
export const PROTOCOL_VERSION = 1;
/** `Hola.juego` value for connections that only watch (no body). */
export const OBSERVER = "observador";
/** `techo` value meaning open sky. */
export const NO_CEILING = 32767;

export const WEAPON_FIST = 0;
export const WEAPON_PISTOL = 1;
export const WEAPON_SHOTGUN = 2;

/** Grid of 1 m cells centred on the origin; layers row by row (z, then x). */
export interface Terreno {
  /** Point of the original map used as the neutral origin, in its own units. */
  origen: [number, number, number];
  /** Cells per side. */
  lado: number;
  /** Floor height of every cell (metres). On a wall cell, the top of the wall. */
  alturas: number[];
  /** Water surface. */
  agua: boolean[];
  /** Sea level. */
  nivel_mar: number;
  /** "game:map", e.g. "doom:E1M1", "openarena:oa_dm1", "signet:nexo". */
  fuente?: string;
  /** Ceiling height; NO_CEILING = open sky. */
  techo?: number[];
  /** Cells that cannot be walked on (walls, void). */
  solido?: boolean[];
  /** Material of each cell: index into `paleta`. */
  material?: number[];
  /** Namespaced neutral materials: "core:piedra" (stone), "core:metal"… */
  paleta?: string[];
  /** Ambient light 0–255. */
  luz?: number[];
  /** Spawn points (x, z). */
  apariciones?: [number, number][];
  /** Cells that see the sky (while `techo` keeps the real height for façades). */
  cielo?: boolean[];
}

export interface JugadorNet {
  id: number;
  /** Feet position in neutral metres [x, y, z]. */
  pos: [number, number, number];
  /** Game it joined with ("doom", "minecraft", "bot"…). */
  juego: string;
  yaw?: number;
  /** Health 0–100. */
  vida?: number;
  /** Armour 0–100. */
  armadura?: number;
  /** Current weapon. */
  arma?: number;
  /** Ammo of the current weapon. */
  municion?: number;
  frags?: number;
  /** Alive (defaults to true). */
  vivo?: boolean;
  /** Last numbered command the server applied for this player. */
  seq?: number;
}

/** Shot (Disparo), damage (Danio), death (Muerte), respawn (Reaparicion). */
export type Evento =
  | { Disparo: { de: number; arma: number; desde: [number, number, number]; hasta: [number, number, number] } }
  | { Danio: { a: number; de: number; cantidad: number; desde: [number, number, number] } }
  | { Muerte: { victima: number; autor: number; arma: number } }
  | { Reaparicion: { id: number } };

/** What the player wants to do right now. */
export interface Intencion {
  /** Forward −1…1. */
  avance: number;
  /** Strafe −1 (left) … 1 (right). */
  lateral: number;
  yaw: number;
  /** Run. */
  correr: boolean;
  /** Fire. */
  disparar: boolean;
  /** Use (reserved). */
  usar: boolean;
  /** Weapon. */
  arma: number;
  /** Command number; 0 = unnumbered (the server applies the latest intent every tick). */
  seq?: number;
}

export type DelCliente =
  | { Hola: { juego: string; version?: number } }
  | { Posicion: { x: number; z: number } }
  | { Intencion: Intencion };

export type DelServidor =
  | { Bienvenida: { tu_id: number; terreno: Terreno; version?: number; modo?: string; mapa?: unknown } }
  | { Estado: { jugadores: JugadorNet[]; eventos?: Evento[] } };

/** Cell that contains (x, z), exactly as in Rust. */
export function cellIndex(t: Terreno, x: number, z: number): number {
  const half = Math.floor(t.lado / 2);
  const i = Math.min(Math.max(Math.floor(x + half), 0), t.lado - 1);
  const j = Math.min(Math.max(Math.floor(z + half), 0), t.lado - 1);
  return j * t.lado + i;
}

export function heightAt(t: Terreno, x: number, z: number): number {
  return t.alturas[cellIndex(t, x, z)];
}

export function walkable(t: Terreno, x: number, z: number): boolean {
  const limit = Math.floor(t.lado / 2) - 1;
  return Math.abs(x) <= limit && Math.abs(z) <= limit && !(t.solido?.[cellIndex(t, x, z)] ?? false);
}
