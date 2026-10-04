//! Modo "doom:deathmatch": movimiento, armas y disparos, sin gráficos. Lo
//! ejecuta el servidor (autoridad) y el cliente (para predecir su propio
//! movimiento sin esperar al servidor). Los valores salen del código fuente
//! de Doom publicado por id Software (GPL) y se pasan a metros (32 unidades ≈ 1 m).

use crate::protocolo::{ARMA_ESCOPETA, ARMA_PISTOLA, ARMA_PUNO, Intencion, Terreno};

pub const MODO: &str = "doom:deathmatch";

/// Velocidad máxima andando y corriendo: forwardmove 25/50 con la fricción
/// de Doom (0xE800) dan ~8,3 y ~16,7 unidades por tic de 1/35 s.
pub const VELOCIDAD_ANDAR: f32 = 9.0;
pub const VELOCIDAD_CORRER: f32 = 18.0;
/// Radio del jugador: 16 unidades.
pub const RADIO: f32 = 0.5;
/// Altura de los ojos: 41 unidades.
pub const ALTURA_OJOS: f32 = 1.3;
/// Escalón máximo: 24 unidades, redondeado a la rejilla de 1 m.
pub const PASO_MAX: f32 = 1.0;
pub const VIDA_INICIAL: i32 = 100;
pub const VIDA_BOT: i32 = 20;
pub const BALAS_INICIALES: i32 = 50;
/// En Doom la escopeta se recoge del mapa; mientras no haya objetos, se da al aparecer.
pub const CARTUCHOS_INICIALES: i32 = 20;

/// Dirección hacia la que mira un yaw (misma convención que la cámara: 0 = -z).
pub fn adelante(yaw: f32) -> (f32, f32) {
    (-yaw.sin(), -yaw.cos())
}

pub fn derecha(yaw: f32) -> (f32, f32) {
    (yaw.cos(), -yaw.sin())
}

/// Si el cuerpo cabe en (x, z) viniendo de una altura `y` (no más de un escalón).
pub fn cabe(t: &Terreno, x: f32, z: f32, y: f32) -> bool {
    let r = RADIO * 0.7;
    [(r, r), (r, -r), (-r, r), (-r, -r)]
        .iter()
        .all(|(dx, dz)| t.transitable(x + dx, z + dz) && t.altura(x + dx, z + dz) - y <= PASO_MAX)
}

/// Distancia máxima de cada subpaso. El escalón se mide desde el subpaso anterior,
/// así que el servidor (20 Hz) y el cliente (cada fotograma) deben avanzar en los
/// mismos trozos o uno sube la escalera y el otro se queda abajo.
const SUBPASO: f32 = 0.1;

/// Movimiento durante `dt`, en subpasos de como mucho `SUBPASO` metros.
pub fn mover(t: &Terreno, pos: (f32, f32), y: f32, i: &Intencion, dt: f32) -> ((f32, f32), f32) {
    let v = if i.correr { VELOCIDAD_CORRER } else { VELOCIDAD_ANDAR };
    let n = ((v * dt / SUBPASO).ceil() as u32).max(1);
    let (mut p, mut y) = (pos, y);
    for _ in 0..n {
        (p, y) = subpaso(t, p, y, i, dt / n as f32);
    }
    (p, y)
}

/// Un subpaso: primero en x y luego en z, para deslizarse por las paredes.
fn subpaso(t: &Terreno, pos: (f32, f32), y: f32, i: &Intencion, dt: f32) -> ((f32, f32), f32) {
    let (fx, fz) = adelante(i.yaw);
    let (rx, rz) = derecha(i.yaw);
    let (mut dx, mut dz) = (fx * i.avance + rx * i.lateral, fz * i.avance + rz * i.lateral);
    let largo = (dx * dx + dz * dz).sqrt();
    if largo > 1.0 {
        dx /= largo;
        dz /= largo;
    }
    let v = if i.correr { VELOCIDAD_CORRER } else { VELOCIDAD_ANDAR } * dt;
    let mut p = pos;
    if cabe(t, p.0 + dx * v, p.1, y) {
        p.0 += dx * v;
    }
    let y1 = t.altura(p.0, p.1);
    if cabe(t, p.0, p.1 + dz * v, y1) {
        p.1 += dz * v;
    }
    (p, t.altura(p.0, p.1))
}

pub struct Arma {
    /// Daño por perdigón = base × (1..=dados), como `damage = 5*(P_Random()%3+1)`.
    pub base: i32,
    pub dados: i32,
    pub perdigones: u32,
    /// Dispersión horizontal máxima en radianes.
    pub dispersion: f32,
    pub alcance: f32,
    /// Segundos entre disparos.
    pub cadencia: f32,
}

pub fn arma(id: u8) -> Arma {
    match id {
        // Puño: (P_Random()%10+1)<<1, alcance MELEERANGE = 64 unidades.
        ARMA_PUNO => Arma { base: 2, dados: 10, perdigones: 1, dispersion: 0.0, alcance: 2.0, cadencia: 0.55 },
        // Escopeta: 7 perdigones, ~37 tics entre disparos.
        ARMA_ESCOPETA => Arma { base: 5, dados: 3, perdigones: 7, dispersion: 0.098, alcance: 64.0, cadencia: 1.05 },
        // Pistola: alcance MISSILERANGE = 2048 unidades.
        _ => Arma { base: 5, dados: 3, perdigones: 1, dispersion: 0.03, alcance: 64.0, cadencia: 0.4 },
    }
}

/// Arma del zombi (A_PosAttack): 3 × (1..5).
pub fn arma_zombi() -> Arma {
    Arma { base: 3, dados: 5, perdigones: 1, dispersion: 0.12, alcance: 40.0, cadencia: 1.6 }
}

pub fn es_arma_valida(id: u8) -> bool {
    matches!(id, ARMA_PUNO | ARMA_PISTOLA | ARMA_ESCOPETA)
}

/// Distancia hasta la primera pared en esa dirección, a la altura `y` (2.5D,
/// como el trazado de disparos de Doom en el plano).
pub fn hasta_pared(t: &Terreno, desde: (f32, f32), y: f32, dir: (f32, f32), alcance: f32) -> f32 {
    let paso = 0.1;
    let mut d = 0.0;
    while d < alcance {
        let (x, z) = (desde.0 + dir.0 * d, desde.1 + dir.1 * d);
        if !t.transitable(x, z) || t.altura(x, z) > y {
            return d;
        }
        d += paso;
    }
    alcance
}

/// Si un disparo desde `desde` en `dir` pasa a menos de `RADIO` de `blanco`
/// antes de `max`: devuelve la distancia a la que lo toca.
pub fn toca(desde: (f32, f32), dir: (f32, f32), blanco: (f32, f32), max: f32) -> Option<f32> {
    let (bx, bz) = (blanco.0 - desde.0, blanco.1 - desde.1);
    let a_lo_largo = bx * dir.0 + bz * dir.1;
    if a_lo_largo <= 0.0 || a_lo_largo > max {
        return None;
    }
    let lateral = (bx * dir.1 - bz * dir.0).abs();
    (lateral < RADIO).then_some(a_lo_largo)
}
