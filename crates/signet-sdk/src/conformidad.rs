//! Conformance suite: checks an importer or translator must pass before it is
//! published. Call them from the translator's tests, or run the `conformidad`
//! example on a terrain exported to JSON:
//!
//! ```text
//! cargo run --example conformidad -- my_map.json
//! ```

use crate::protocolo::{Intencion, SIN_TECHO, Terreno, VERSION_PROTOCOLO};
use crate::reglas::{TIC, doom as reglas};
use crate::traductor::Manifiesto;

/// A failed check.
#[derive(Debug, Clone, PartialEq)]
pub struct Fallo {
    /// Stable code (for documentation and CI): "T01", "M02"…
    pub codigo: &'static str,
    pub mensaje: String,
}

fn fallo(codigo: &'static str, mensaje: impl Into<String>) -> Fallo {
    Fallo { codigo, mensaje: mensaje.into() }
}

/// Checks the shape and consistency of a neutral terrain.
///
/// * T01 sizes: every layer has `side²` cells (or is empty if optional).
/// * T02 `fuente` has the form `game:map`.
/// * T03 materials: indices inside `paleta` and namespaced names (`core:piedra`).
/// * T04 spawn points inside the map and on walkable cells.
/// * T05 ceilings above the floor on walkable cells.
/// * T06 at least one walkable cell.
/// * T07 the terrain survives a JSON round trip unchanged.
pub fn comprobar_terreno(t: &Terreno) -> Vec<Fallo> {
    let mut f = Vec::new();
    let n = t.lado * t.lado;
    if t.lado < 4 {
        f.push(fallo("T01", format!("side {} is too small", t.lado)));
        return f;
    }
    if t.alturas.len() != n {
        f.push(fallo("T01", format!("alturas has {} cells and should have {n}", t.alturas.len())));
        return f;
    }
    for (nombre, largo) in [
        ("techo", t.techo.len()),
        ("solido", t.solido.len()),
        ("material", t.material.len()),
        ("luz", t.luz.len()),
        ("cielo", t.cielo.len()),
    ] {
        if largo != 0 && largo != n {
            f.push(fallo("T01", format!("{nombre} has {largo} cells: it must have 0 or {n}")));
        }
    }
    if t.agua.len() != n {
        f.push(fallo("T01", format!("agua is mandatory and has {} cells instead of {n}", t.agua.len())));
    }
    match t.fuente.split_once(':') {
        Some((j, m)) if !j.is_empty() && !m.is_empty() => {}
        _ => f.push(fallo("T02", format!("fuente \"{}\" does not have the form game:map", t.fuente))),
    }
    if let Some(m) = t.material.iter().find(|m| **m as usize >= t.paleta.len().max(1)) {
        f.push(fallo("T03", format!("material {m} is outside the palette ({} names)", t.paleta.len())));
    }
    for p in t.paleta.iter().filter(|p| !p.contains(':')) {
        f.push(fallo("T03", format!("material \"{p}\" has no namespace (use core:… or yourgame:…)")));
    }
    for (k, a) in t.apariciones.iter().enumerate() {
        if !t.transitable(a[0], a[1]) {
            f.push(fallo("T04", format!("spawn point {k} at ({:.1}, {:.1}) is not walkable", a[0], a[1])));
        }
    }
    if !t.techo.is_empty() {
        let mal = (0..n).filter(|&k| !t.es_solido(k) && t.techo[k] != SIN_TECHO && t.techo[k] <= t.alturas[k]).count();
        if mal > 0 {
            f.push(fallo("T05", format!("{mal} walkable cells have their ceiling at or below the floor")));
        }
    }
    if (0..n).all(|k| t.es_solido(k)) {
        f.push(fallo("T06", "no walkable cell"));
    }
    match serde_json::to_string(t).and_then(|s| serde_json::from_str::<Terreno>(&s)) {
        Ok(v) if v.alturas == t.alturas && v.solido == t.solido && v.fuente == t.fuente => {}
        Ok(_) => f.push(fallo("T07", "the terrain changes after a JSON round trip")),
        Err(e) => f.push(fallo("T07", format!("cannot be serialised: {e}"))),
    }
    f
}

/// Checks that movement on this terrain is deterministic: the same commands
/// from the same position always give the same result, even when computed
/// separately (which is what keeps client and server in agreement).
///
/// * R01 two identical simulations end at the same point.
/// * R02 from every spawn point a body can walk at least one metre in some direction.
pub fn comprobar_reglas(t: &Terreno) -> Vec<Fallo> {
    let mut f = Vec::new();
    let inicios: Vec<(f32, f32)> = if t.apariciones.is_empty() { vec![(0.0, 0.0)] } else { t.apariciones.iter().map(|a| (a[0], a[1])).collect() };
    for (k, &p0) in inicios.iter().enumerate() {
        let comandos: Vec<Intencion> = (0..200)
            .map(|s| Intencion { avance: 1.0, lateral: ((s / 25) % 2) as f32, yaw: s as f32 * 0.07, correr: s % 3 == 0, seq: s + 1, ..Default::default() })
            .collect();
        let simular = || comandos.iter().fold((p0, t.altura(p0.0, p0.1)), |(p, y), c| reglas::mover(t, p, y, c, TIC));
        if simular() != simular() {
            f.push(fallo("R01", format!("non-deterministic movement from spawn point {k}")));
        }
        let y0 = t.altura(p0.0, p0.1);
        let anda = (0..8).any(|d| {
            let c = Intencion { avance: 1.0, yaw: d as f32 * std::f32::consts::FRAC_PI_4, ..Default::default() };
            let (p, _) = (0..4).fold((p0, y0), |(p, y), _| reglas::mover(t, p, y, &c, TIC));
            ((p.0 - p0.0).powi(2) + (p.1 - p0.1).powi(2)).sqrt() >= 1.0
        });
        if !anda {
            f.push(fallo("R02", format!("from spawn point {k} a body cannot walk in any direction")));
        }
    }
    f
}

/// Checks a translator's manifest.
///
/// * M01 protocol supported by this SDK.
/// * M02 semver version (`MAJOR.MINOR.PATCH`).
/// * M03 provides at least one capability.
/// * M04 game id in lowercase, with no spaces or `:`.
pub fn comprobar_manifiesto(m: &Manifiesto) -> Vec<Fallo> {
    let mut f = Vec::new();
    if m.protocolo != VERSION_PROTOCOLO {
        f.push(fallo("M01", format!("protocol {} is not supported (this SDK speaks Signet/{VERSION_PROTOCOLO})", m.protocolo)));
    }
    let semver = m.version.split('-').next().unwrap_or("").split('.').filter(|p| p.parse::<u32>().is_ok()).count() == 3;
    if !semver {
        f.push(fallo("M02", format!("version \"{}\" is not semver", m.version)));
    }
    let c = &m.capacidades;
    if !(c.importador || c.mundo || c.entrada || c.presentacion) {
        f.push(fallo("M03", "declares no capability"));
    }
    if m.juego.is_empty() || m.juego.chars().any(|ch| ch.is_uppercase() || ch.is_whitespace() || ch == ':') {
        f.push(fallo("M04", format!("invalid game id \"{}\"", m.juego)));
    }
    f
}
