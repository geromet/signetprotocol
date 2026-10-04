//! Mapas de OpenArena / Quake III (BSP "IBSP" versión 46), sin gráficos:
//!
//! * `cargar`: el servidor convierte el mapa al terreno neutral 2.5D sondeando
//!   los brushes (los volúmenes sólidos con los que colisiona el motor) en
//!   columnas de 1 m: suelo = la superficie sólida más baja con hueco para una
//!   persona; techo = lo primero sólido encima (cielo si ese brush lleva cielo).
//! * `geometria_nativa`: el visor OpenArena dibuja las caras originales
//!   (polígonos, mallas y curvas Bézier) con su textura y la luz horneada.
//!
//! Escala y ejes como Doom: 32 unidades = 1 m; x → x, y (norte) → -z, z (arriba) → y.

use crate::openarena::Paquetes;
use crate::protocolo::{SIN_TECHO, Terreno};

const UNIDADES: f32 = 32.0;
/// Altura libre mínima para que una persona quepa (Quake III: 56 unidades de pie).
const HUECO: f32 = 56.0;
const PASO_SONDEO: f32 = 8.0;
const MARGEN: usize = 4;

const SOLIDO: i32 = 1;
const LAVA: i32 = 8;
const SLIME: i32 = 16;
const AGUA: i32 = 32;
const SURF_CIELO: i32 = 0x4;

fn i32le(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn f32le(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

struct Textura {
    nombre: String,
    flags: i32,
    contenido: i32,
}
struct Plano {
    n: [f32; 3],
    d: f32,
}
struct Nodo {
    plano: usize,
    hijos: [i32; 2],
}
struct Hoja {
    primero: usize,
    cuantos: usize,
}
struct Brush {
    primero: usize,
    cuantos: usize,
    textura: usize,
}
struct Lado {
    plano: usize,
    textura: usize,
}
#[derive(Clone, Copy)]
struct Vertice {
    pos: [f32; 3],
    uv: [f32; 2],
    color: [u8; 4],
}
struct Cara {
    textura: usize,
    tipo: i32,
    vertice: usize,
    n_vertices: usize,
    meshvert: usize,
    n_meshverts: usize,
    tam: [usize; 2],
}

struct Bsp {
    entidades: String,
    texturas: Vec<Textura>,
    planos: Vec<Plano>,
    nodos: Vec<Nodo>,
    hojas: Vec<Hoja>,
    hoja_brushes: Vec<usize>,
    brushes: Vec<Brush>,
    lados: Vec<Lado>,
    vertices: Vec<Vertice>,
    meshverts: Vec<u32>,
    caras: Vec<Cara>,
    min: [f32; 3],
    max: [f32; 3],
}

fn leer_bsp(b: &[u8]) -> Result<Bsp, String> {
    if b.len() < 144 || &b[0..4] != b"IBSP" || i32le(b, 4) != 46 {
        return Err("no es un BSP de Quake III (IBSP v46)".into());
    }
    let lump = |i: usize| -> &[u8] {
        let (o, l) = (i32le(b, 8 + i * 8) as usize, i32le(b, 12 + i * 8) as usize);
        &b[o.min(b.len())..(o + l).min(b.len())]
    };
    let nombre = |d: &[u8], o: usize| d[o..o + 64].iter().take_while(|c| **c != 0).map(|c| *c as char).collect::<String>().to_ascii_lowercase();
    let ent = lump(0);
    let tx = lump(1);
    let texturas = (0..tx.len() / 72).map(|k| Textura { nombre: nombre(tx, k * 72), flags: i32le(tx, k * 72 + 64), contenido: i32le(tx, k * 72 + 68) }).collect();
    let pl = lump(2);
    let planos = (0..pl.len() / 16).map(|k| Plano { n: [f32le(pl, k * 16), f32le(pl, k * 16 + 4), f32le(pl, k * 16 + 8)], d: f32le(pl, k * 16 + 12) }).collect();
    let nd = lump(3);
    let nodos = (0..nd.len() / 36).map(|k| Nodo { plano: i32le(nd, k * 36) as usize, hijos: [i32le(nd, k * 36 + 4), i32le(nd, k * 36 + 8)] }).collect();
    let lf = lump(4);
    let hojas = (0..lf.len() / 48).map(|k| Hoja { primero: i32le(lf, k * 48 + 40) as usize, cuantos: i32le(lf, k * 48 + 44) as usize }).collect();
    let lb = lump(6);
    let hoja_brushes = (0..lb.len() / 4).map(|k| i32le(lb, k * 4) as usize).collect();
    let md = lump(7);
    let (min, max) = if md.len() >= 24 { ([f32le(md, 0), f32le(md, 4), f32le(md, 8)], [f32le(md, 12), f32le(md, 16), f32le(md, 20)]) } else { ([0.0; 3], [0.0; 3]) };
    let br = lump(8);
    let brushes = (0..br.len() / 12).map(|k| Brush { primero: i32le(br, k * 12) as usize, cuantos: i32le(br, k * 12 + 4) as usize, textura: i32le(br, k * 12 + 8) as usize }).collect();
    let bs = lump(9);
    let lados = (0..bs.len() / 8).map(|k| Lado { plano: i32le(bs, k * 8) as usize, textura: i32le(bs, k * 8 + 4) as usize }).collect();
    let vx = lump(10);
    let vertices = (0..vx.len() / 44)
        .map(|k| {
            let o = k * 44;
            Vertice { pos: [f32le(vx, o), f32le(vx, o + 4), f32le(vx, o + 8)], uv: [f32le(vx, o + 12), f32le(vx, o + 16)], color: [vx[o + 40], vx[o + 41], vx[o + 42], vx[o + 43]] }
        })
        .collect();
    let mv = lump(11);
    let meshverts = (0..mv.len() / 4).map(|k| i32le(mv, k * 4) as u32).collect();
    let fc = lump(13);
    let caras = (0..fc.len() / 104)
        .map(|k| {
            let o = k * 104;
            Cara {
                textura: i32le(fc, o) as usize,
                tipo: i32le(fc, o + 8),
                vertice: i32le(fc, o + 12) as usize,
                n_vertices: i32le(fc, o + 16) as usize,
                meshvert: i32le(fc, o + 20) as usize,
                n_meshverts: i32le(fc, o + 24) as usize,
                tam: [i32le(fc, o + 96).max(0) as usize, i32le(fc, o + 100).max(0) as usize],
            }
        })
        .collect();
    Ok(Bsp {
        entidades: String::from_utf8_lossy(ent).into_owned(),
        texturas,
        planos,
        nodos,
        hojas,
        hoja_brushes,
        brushes,
        lados,
        vertices,
        meshverts,
        caras,
        min,
        max,
    })
}

/// Lo que hay en un punto: contenidos de los brushes que lo contienen y si alguno es cielo.
struct Contenido {
    bits: i32,
    cielo: bool,
    brush: Option<usize>,
}

impl Bsp {
    fn en(&self, p: [f32; 3]) -> Contenido {
        let mut i: i32 = 0;
        while i >= 0 {
            let nodo = &self.nodos[i as usize];
            let pl = &self.planos[nodo.plano];
            let d = pl.n[0] * p[0] + pl.n[1] * p[1] + pl.n[2] * p[2] - pl.d;
            i = if d >= 0.0 { nodo.hijos[0] } else { nodo.hijos[1] };
        }
        let hoja = &self.hojas[(-(i + 1)) as usize];
        let mut c = Contenido { bits: 0, cielo: false, brush: None };
        for &k in &self.hoja_brushes[hoja.primero..hoja.primero + hoja.cuantos] {
            let b = &self.brushes[k];
            let dentro = self.lados[b.primero..b.primero + b.cuantos].iter().all(|l| {
                let pl = &self.planos[l.plano];
                pl.n[0] * p[0] + pl.n[1] * p[1] + pl.n[2] * p[2] - pl.d <= 0.0
            });
            if dentro {
                let contenido = self.texturas.get(b.textura).map_or(0, |t| t.contenido);
                c.bits |= contenido;
                if contenido & SOLIDO != 0 {
                    c.brush = Some(k);
                    c.cielo |= self.lados[b.primero..b.primero + b.cuantos].iter().any(|l| self.texturas.get(l.textura).is_some_and(|t| t.flags & SURF_CIELO != 0));
                }
            }
        }
        c
    }

    /// Textura de la cara superior (la que mira hacia arriba) de un brush.
    fn textura_superior(&self, brush: usize) -> &str {
        let b = &self.brushes[brush];
        self.lados[b.primero..b.primero + b.cuantos]
            .iter()
            .filter(|l| self.planos[l.plano].n[2] > 0.7)
            .find_map(|l| self.texturas.get(l.textura))
            .map_or("", |t| t.nombre.as_str())
    }

    /// Valor de una clave de las entidades de un tipo ("origin" de los "info_player_deathmatch"…).
    fn entidades_de(&self, clase: &str) -> Vec<[f32; 3]> {
        let mut v = Vec::new();
        for bloque in self.entidades.split('}') {
            let mut pares = Vec::new();
            let partes: Vec<&str> = bloque.split('"').collect();
            for k in (1..partes.len().saturating_sub(2)).step_by(4) {
                pares.push((partes[k], partes[k + 2]));
            }
            if pares.iter().any(|(k, val)| *k == "classname" && *val == clase)
                && let Some((_, o)) = pares.iter().find(|(k, _)| *k == "origin")
            {
                let n: Vec<f32> = o.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                if n.len() == 3 {
                    v.push([n[0], n[1], n[2]]);
                }
            }
        }
        v
    }
}

fn abrir(nombre_mapa: &str) -> Result<Bsp, String> {
    let mut pk = Paquetes::abrir()?;
    let datos = pk.leer(&format!("maps/{nombre_mapa}.bsp")).ok_or(format!("OpenArena has no map called {nombre_mapa}"))?;
    leer_bsp(&datos)
}

/// Clasifica una textura de Quake III en un material neutral.
fn material_neutral(nombre: &str, contenido: i32) -> &'static str {
    if contenido & LAVA != 0 || nombre.contains("lava") {
        return "core:lava";
    }
    if contenido & SLIME != 0 || nombre.contains("slime") {
        return "core:toxico";
    }
    if contenido & AGUA != 0 || nombre.contains("water") || nombre.contains("pool") {
        return "core:agua";
    }
    let tiene = |p: &[&str]| p.iter().any(|x| nombre.contains(x));
    if tiene(&["metal", "clang", "grate", "tin", "pewter", "rust"]) {
        "core:metal"
    } else if tiene(&["tech", "atech", "comp", "proto", "light"]) {
        "core:tecnologia"
    } else if tiene(&["brick", "brik"]) {
        "core:ladrillo"
    } else if tiene(&["wood", "plank"]) {
        "core:madera"
    } else if tiene(&["grass", "moss", "ground", "dirt", "sand"]) {
        "core:hierba"
    } else if tiene(&["gothic", "block", "stone", "rock", "marble", "concrete", "cement"]) {
        "core:piedra"
    } else {
        "core:baldosa"
    }
}

/// Convierte un mapa de OpenArena al terreno neutral (para el servidor).
pub fn cargar(nombre_mapa: &str) -> Result<Terreno, String> {
    let bsp = abrir(nombre_mapa)?;
    let apariciones_q3 = bsp.entidades_de("info_player_deathmatch");
    let origen = *apariciones_q3.first().ok_or("el mapa no tiene info_player_deathmatch")?;
    let (min, max) = (bsp.min, bsp.max);
    let alcance = [origen[0] - min[0], max[0] - origen[0], origen[1] - min[1], max[1] - origen[1]].into_iter().fold(0.0f32, f32::max);
    let mitad = (alcance / UNIDADES).ceil() as usize + MARGEN;
    let lado = mitad * 2;
    let n = lado * lado;
    let centro = |i: usize, j: usize| (origen[0] + (i as f32 - mitad as f32 + 0.5) * UNIDADES, origen[1] - (j as f32 - mitad as f32 + 0.5) * UNIDADES);

    // Suelo y techo de cada columna, en unidades de Quake.
    let pasos = ((max[2] - min[2]) / PASO_SONDEO).ceil() as usize + 2;
    let necesarios = (HUECO / PASO_SONDEO).ceil() as usize;
    let mut columnas: Vec<Option<(f32, f32, bool, &'static str)>> = vec![None; n];
    for j in 0..lado {
        for i in 0..lado {
            let (x, y) = centro(i, j);
            if x < min[0] || x > max[0] || y < min[1] || y > max[1] {
                continue;
            }
            let muestras: Vec<Contenido> = (0..pasos).map(|s| bsp.en([x, y, min[2] + s as f32 * PASO_SONDEO])).collect();
            let solido = |s: usize| muestras.get(s).is_none_or(|c| c.bits & SOLIDO != 0);
            // El primer suelo (sólido con hueco libre encima) desde abajo.
            let Some(s) = (0..pasos.saturating_sub(necesarios + 1)).find(|&s| solido(s) && (1..=necesarios).all(|k| !solido(s + k))) else { continue };
            let suelo = min[2] + (s as f32 + 1.0) * PASO_SONDEO;
            let encima = (s + 1..pasos).find(|&k| solido(k));
            let (techo, cielo) = match encima {
                Some(k) => (min[2] + k as f32 * PASO_SONDEO, muestras[k].cielo),
                None => (max[2], true),
            };
            let liquido = muestras[s + 1].bits & (LAVA | SLIME | AGUA);
            let textura = muestras[s].brush.map_or("", |b| bsp.textura_superior(b));
            columnas[j * lado + i] = Some((suelo, techo, cielo, material_neutral(textura, liquido)));
        }
    }

    let suelo_origen = columnas[mitad * lado + mitad].map_or(origen[2] - 24.0, |c| c.0);
    let metros = |u: f32| ((u - suelo_origen) / UNIDADES).round() as i16;
    let mut paleta: Vec<String> = Vec::new();
    let mut indice = |m: &str| -> u8 {
        if let Some(i) = paleta.iter().position(|p| p == m) {
            return i as u8;
        }
        paleta.push(m.to_string());
        (paleta.len() - 1) as u8
    };
    let (mut alturas, mut techo, mut solido, mut material, mut cielo) = (vec![0i16; n], vec![SIN_TECHO; n], vec![true; n], vec![0u8; n], vec![false; n]);
    for k in 0..n {
        if let Some((s, t, c, m)) = columnas[k] {
            alturas[k] = metros(s);
            techo[k] = metros(t);
            cielo[k] = c;
            solido[k] = techo[k] <= alturas[k];
            material[k] = indice(m);
        }
    }
    // Paredes: las celdas sólidas toman la altura del techo vecino más alto, como en Doom.
    let antes = solido.clone();
    let pared = indice("core:pared");
    let mut base = i16::MAX;
    for k in (0..n).filter(|k| !antes[*k]) {
        base = base.min(alturas[k]);
    }
    for j in 0..lado {
        for i in 0..lado {
            let k = j * lado + i;
            if !antes[k] {
                continue;
            }
            let mut tope = None;
            for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                let (vi, vj) = (i as i32 + di, j as i32 + dj);
                if vi >= 0 && vj >= 0 && (vi as usize) < lado && (vj as usize) < lado && !antes[vj as usize * lado + vi as usize] {
                    tope = Some(tope.unwrap_or(i16::MIN).max(techo[vj as usize * lado + vi as usize]));
                }
            }
            alturas[k] = tope.unwrap_or(base.saturating_sub(2));
            techo[k] = alturas[k];
            material[k] = pared;
        }
    }
    let apariciones = apariciones_q3.iter().map(|o| [(o[0] - origen[0]) / UNIDADES, -(o[1] - origen[1]) / UNIDADES]).collect();
    let abiertas = (0..n).filter(|k| !solido[*k]).count();
    eprintln!("OpenArena {nombre_mapa}: {lado}x{lado} cells, {abiertas} walkable, {} spawn points", apariciones_q3.len());
    Ok(Terreno {
        origen: [origen[0] as i32, suelo_origen as i32, origen[1] as i32],
        lado,
        alturas,
        agua: vec![false; n],
        nivel_mar: base.saturating_sub(10),
        fuente: format!("openarena:{nombre_mapa}"),
        techo,
        solido,
        material,
        paleta,
        luz: vec![200; n],
        apariciones,
        cielo,
    })
}

/// Una cara del mapa original lista para dibujar.
pub struct SuperficieQ3 {
    pub textura: String,
    /// Triángulos (de tres en tres) en coordenadas neutrales.
    pub puntos: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    /// Luz horneada del vértice (0–1).
    pub color: Vec<[f32; 4]>,
}

/// Las caras del mapa de OpenArena, con el mismo origen que `cargar`.
pub fn geometria_nativa(nombre_mapa: &str, origen: [i32; 3]) -> Result<Vec<SuperficieQ3>, String> {
    let bsp = abrir(nombre_mapa)?;
    let o = [origen[0] as f32, origen[2] as f32, origen[1] as f32];
    let neutral = |p: [f32; 3]| [(p[0] - o[0]) / UNIDADES, (p[2] - o[2]) / UNIDADES, -(p[1] - o[1]) / UNIDADES];
    let mut por_textura: std::collections::HashMap<usize, SuperficieQ3> = std::collections::HashMap::new();
    for c in &bsp.caras {
        let Some(t) = bsp.texturas.get(c.textura) else { continue };
        let invisible = ["sky", "caulk", "nodraw", "clip", "trigger", "hint", "skip", "fog"].iter().any(|x| t.nombre.contains(x));
        if invisible || t.flags & SURF_CIELO != 0 {
            continue;
        }
        let mut tris: Vec<Vertice> = Vec::new();
        match c.tipo {
            // Polígono o malla: los índices de triángulo vienen en meshverts.
            1 | 3 => {
                for k in 0..c.n_meshverts {
                    let v = c.vertice + bsp.meshverts[c.meshvert + k] as usize;
                    if let Some(v) = bsp.vertices.get(v) {
                        tris.push(*v);
                    }
                }
            }
            // Curva: parches Bézier bicuadráticos de 3×3 puntos de control.
            2 => {
                let (w, h) = (c.tam[0], c.tam[1]);
                if w < 3 || h < 3 {
                    continue;
                }
                let control = &bsp.vertices[c.vertice..(c.vertice + c.n_vertices).min(bsp.vertices.len())];
                let nivel = 6;
                for py in (0..h - 1).step_by(2) {
                    for px in (0..w - 1).step_by(2) {
                        let punto = |u: f32, v: f32| {
                            let b = |t: f32| [(1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t];
                            let (bu, bv) = (b(u), b(v));
                            let mut r = Vertice { pos: [0.0; 3], uv: [0.0; 2], color: [0; 4] };
                            let mut col = [0.0f32; 4];
                            for (j, wv) in bv.iter().enumerate() {
                                for (i, wu) in bu.iter().enumerate() {
                                    let Some(cp) = control.get((py + j) * w + px + i) else { continue };
                                    let peso = wu * wv;
                                    for a in 0..3 {
                                        r.pos[a] += cp.pos[a] * peso;
                                    }
                                    r.uv[0] += cp.uv[0] * peso;
                                    r.uv[1] += cp.uv[1] * peso;
                                    for a in 0..4 {
                                        col[a] += cp.color[a] as f32 * peso;
                                    }
                                }
                            }
                            r.color = col.map(|x| x.clamp(0.0, 255.0) as u8);
                            r
                        };
                        for y in 0..nivel {
                            for x in 0..nivel {
                                let (u0, u1) = (x as f32 / nivel as f32, (x + 1) as f32 / nivel as f32);
                                let (v0, v1) = (y as f32 / nivel as f32, (y + 1) as f32 / nivel as f32);
                                let (a, b, cc, d) = (punto(u0, v0), punto(u1, v0), punto(u1, v1), punto(u0, v1));
                                tris.extend([a, cc, b, a, d, cc]);
                            }
                        }
                    }
                }
            }
            _ => continue,
        }
        let s = por_textura.entry(c.textura).or_insert_with(|| SuperficieQ3 { textura: t.nombre.clone(), puntos: Vec::new(), uv: Vec::new(), color: Vec::new() });
        for v in tris {
            s.puntos.push(neutral(v.pos));
            s.uv.push(v.uv);
            // La luz horneada de Quake es oscura; se aclara un poco para verse bien sin lightmaps.
            s.color.push([v.color[0] as f32 / 255.0 * 1.6, v.color[1] as f32 / 255.0 * 1.6, v.color[2] as f32 / 255.0 * 1.6, 1.0]);
        }
    }
    Ok(por_textura.into_values().collect())
}
