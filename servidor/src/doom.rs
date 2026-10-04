//! Importador de mundos de Doom: lee un mapa de un WAD original (por ahora el
//! shareware oficial de id Software, `datos/doom/DOOM1.WAD`) y lo convierte
//! al terreno neutral. Doom es 2.5D: cada sector tiene altura de suelo y de
//! techo, así que encaja en una rejilla de columnas con techo.
//!
//! Escala: 1 celda = 32 unidades de Doom ≈ 1 m (Doomguy mide 56 unidades).
//! Ejes: x de Doom → x neutral; y de Doom (norte) → -z neutral.

use std::collections::HashMap;
use std::path::Path;

use crate::protocolo::{SIN_TECHO, Terreno};

pub const WAD: &str = "datos/doom/DOOM1.WAD";
const UNIDADES_POR_CELDA: f64 = 32.0;
/// Margen de celdas vacías alrededor del mapa.
const MARGEN: usize = 4;

pub struct Lump<'a> {
    pub nombre: String,
    pub datos: &'a [u8],
}

pub fn i16le(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}
pub fn u16le(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
pub fn i32le(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
pub fn nombre8(b: &[u8], o: usize) -> String {
    b[o..o + 8].iter().take_while(|c| **c != 0).map(|c| c.to_ascii_uppercase() as char).collect()
}

pub fn lumps(wad: &[u8]) -> Result<Vec<Lump<'_>>, String> {
    if wad.len() < 12 || !(&wad[0..4] == b"IWAD" || &wad[0..4] == b"PWAD") {
        return Err("no es un archivo WAD".into());
    }
    let n = i32le(wad, 4) as usize;
    let dir = i32le(wad, 8) as usize;
    (0..n)
        .map(|k| {
            let e = dir + k * 16;
            let (pos, tam) = (i32le(wad, e) as usize, i32le(wad, e + 4) as usize);
            let datos = wad.get(pos..pos + tam).ok_or("WAD truncado")?;
            Ok(Lump { nombre: nombre8(wad, e + 8), datos })
        })
        .collect()
}

struct Linea {
    v1: usize,
    v2: usize,
    derecha: Option<usize>,
    izquierda: Option<usize>,
}
struct Lado {
    desplaza_x: f64,
    desplaza_y: f64,
    arriba: String,
    abajo: String,
    medio: String,
    sector: usize,
}
struct Sector {
    suelo: f64,
    techo: f64,
    flat_suelo: String,
    flat_techo: String,
    luz: u8,
}
struct Nodo {
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
    hijos: [u16; 2],
}

struct Mapa {
    vertices: Vec<(f64, f64)>,
    lineas: Vec<Linea>,
    lados: Vec<Lado>,
    sectores: Vec<Sector>,
    segs_linea: Vec<(usize, usize)>,
    segs_vertices: Vec<(usize, usize)>,
    subsectores: Vec<usize>,
    subsectores_largo: Vec<usize>,
    nodos: Vec<Nodo>,
    cosas: Vec<(f64, f64, u16)>,
}

fn leer_mapa(wad: &[u8], nombre: &str) -> Result<Mapa, String> {
    let todos = lumps(wad)?;
    let inicio = todos.iter().position(|l| l.nombre == nombre).ok_or(format!("the WAD has no map called {nombre}"))?;
    let lump = |n: &str| -> Result<&[u8], String> {
        todos[inicio + 1..(inicio + 11).min(todos.len())]
            .iter()
            .find(|l| l.nombre == n)
            .map(|l| l.datos)
            .ok_or(format!("{n} is missing in {nombre}"))
    };
    let lado_opt = |v: u16| (v != 0xFFFF).then_some(v as usize);

    let vx = lump("VERTEXES")?;
    let vertices = (0..vx.len() / 4).map(|k| (i16le(vx, k * 4) as f64, i16le(vx, k * 4 + 2) as f64)).collect();
    let ld = lump("LINEDEFS")?;
    let lineas = (0..ld.len() / 14)
        .map(|k| {
            let o = k * 14;
            Linea {
                v1: u16le(ld, o) as usize,
                v2: u16le(ld, o + 2) as usize,
                derecha: lado_opt(u16le(ld, o + 10)),
                izquierda: lado_opt(u16le(ld, o + 12)),
            }
        })
        .collect();
    let sd = lump("SIDEDEFS")?;
    let lados = (0..sd.len() / 30)
        .map(|k| {
            let o = k * 30;
            Lado {
                desplaza_x: i16le(sd, o) as f64,
                desplaza_y: i16le(sd, o + 2) as f64,
                arriba: nombre8(sd, o + 4),
                abajo: nombre8(sd, o + 12),
                medio: nombre8(sd, o + 20),
                sector: u16le(sd, o + 28) as usize,
            }
        })
        .collect();
    let se = lump("SECTORS")?;
    let sectores = (0..se.len() / 26)
        .map(|k| {
            let o = k * 26;
            Sector {
                suelo: i16le(se, o) as f64,
                techo: i16le(se, o + 2) as f64,
                flat_suelo: nombre8(se, o + 4),
                flat_techo: nombre8(se, o + 12),
                luz: i16le(se, o + 20).clamp(0, 255) as u8,
            }
        })
        .collect();
    let sg = lump("SEGS")?;
    let segs_linea = (0..sg.len() / 12).map(|k| (u16le(sg, k * 12 + 6) as usize, u16le(sg, k * 12 + 8) as usize)).collect();
    let segs_vertices = (0..sg.len() / 12).map(|k| (u16le(sg, k * 12) as usize, u16le(sg, k * 12 + 2) as usize)).collect();
    let ss = lump("SSECTORS")?;
    let subsectores = (0..ss.len() / 4).map(|k| u16le(ss, k * 4 + 2) as usize).collect();
    let subsectores_largo = (0..ss.len() / 4).map(|k| u16le(ss, k * 4) as usize).collect();
    let nd = lump("NODES")?;
    let nodos = (0..nd.len() / 28)
        .map(|k| {
            let o = k * 28;
            Nodo {
                x: i16le(nd, o) as f64,
                y: i16le(nd, o + 2) as f64,
                dx: i16le(nd, o + 4) as f64,
                dy: i16le(nd, o + 6) as f64,
                hijos: [u16le(nd, o + 24), u16le(nd, o + 26)],
            }
        })
        .collect();
    let th = lump("THINGS")?;
    let cosas = (0..th.len() / 10)
        .map(|k| (i16le(th, k * 10) as f64, i16le(th, k * 10 + 2) as f64, u16le(th, k * 10 + 6)))
        .collect();
    Ok(Mapa { vertices, lineas, lados, sectores, segs_linea, segs_vertices, subsectores, subsectores_largo, nodos, cosas })
}

impl Mapa {
    fn sector_de_lado(&self, lado: Option<usize>) -> Option<usize> {
        lado.map(|s| self.lados[s].sector)
    }

    /// Sector cuyo subsector contiene el punto, recorriendo el árbol BSP como
    /// `R_PointInSubsector` del código fuente de Doom (GPL).
    fn sector_bsp(&self, x: f64, y: f64) -> usize {
        let mut n = self.nodos.len() - 1;
        loop {
            let nodo = &self.nodos[n];
            let lado = if nodo.dx == 0.0 {
                if x <= nodo.x { (nodo.dy > 0.0) as usize } else { (nodo.dy < 0.0) as usize }
            } else if nodo.dy == 0.0 {
                if y <= nodo.y { (nodo.dx < 0.0) as usize } else { (nodo.dx > 0.0) as usize }
            } else {
                let izquierda = nodo.dy * (x - nodo.x);
                let derecha = (y - nodo.y) * nodo.dx;
                (derecha >= izquierda) as usize
            };
            let hijo = nodo.hijos[lado];
            if hijo & 0x8000 != 0 {
                let seg = self.subsectores[(hijo & 0x7FFF) as usize];
                let (linea, sentido) = self.segs_linea[seg];
                let l = &self.lineas[linea];
                let lado = if sentido == 0 { l.derecha } else { l.izquierda };
                return lado.map(|s| self.lados[s].sector).unwrap_or(0);
            }
            n = hijo as usize;
        }
    }

    /// El BSP cubre todo el plano; esto descarta los puntos del vacío exterior
    /// con un rayo contra los bordes del sector (regla par-impar).
    fn dentro_de(&self, sector: usize, x: f64, y: f64) -> bool {
        let mut dentro = false;
        for l in &self.lineas {
            let (a, b) = (self.sector_de_lado(l.derecha), self.sector_de_lado(l.izquierda));
            if (a == Some(sector)) == (b == Some(sector)) {
                continue;
            }
            let (x1, y1) = self.vertices[l.v1];
            let (x2, y2) = self.vertices[l.v2];
            if (y1 > y) != (y2 > y) && x < x1 + (y - y1) * (x2 - x1) / (y2 - y1) {
                dentro = !dentro;
            }
        }
        dentro
    }

    fn sector_en(&self, x: f64, y: f64) -> Option<usize> {
        let s = self.sector_bsp(x, y);
        self.dentro_de(s, x, y).then_some(s)
    }
}

/// Clasifica una textura o flat de Doom en un material neutral.
fn material_neutral(nombre: &str) -> &'static str {
    let n = nombre;
    let empieza = |p: &[&str]| p.iter().any(|x| n.starts_with(x));
    if empieza(&["NUKAGE", "SLIME"]) {
        "core:toxico"
    } else if empieza(&["FWATER"]) {
        "core:agua"
    } else if empieza(&["LAVA"]) {
        "core:lava"
    } else if empieza(&["BLOOD"]) {
        "core:sangre"
    } else if empieza(&["STARTAN", "STARG", "STARBR", "STEP", "TLITE", "SUPPORT", "METAL", "SHAWN"]) {
        "core:metal"
    } else if empieza(&["COMP", "PLANET", "SILVER", "TEK", "LITE", "SW1COMP", "SW2COMP"]) {
        "core:tecnologia"
    } else if empieza(&["DOOR", "BIGDOOR", "EXITDOOR"]) {
        "core:puerta"
    } else if empieza(&["WOOD"]) {
        "core:madera"
    } else if empieza(&["BRICK"]) {
        "core:ladrillo"
    } else if empieza(&["GRASS"]) {
        "core:hierba"
    } else if empieza(&["BROWN", "STONE", "GRAY", "ROCK", "ASH", "SP_ROCK", "RROCK", "GRNROCK", "MFLR"]) {
        "core:piedra"
    } else if empieza(&["CEIL"]) {
        "core:techo"
    } else {
        "core:baldosa"
    }
}

/// Puertas cerradas (techo pegado al suelo): se abren hasta el techo vecino
/// más bajo menos 4, como `EV_DoDoor`. Así sus salas quedan conectadas.
fn abrir_puertas(mapa: &mut Mapa) {
    let mut vecinos: HashMap<usize, Vec<usize>> = HashMap::new();
    for l in &mapa.lineas {
        if let (Some(a), Some(b)) = (l.derecha, l.izquierda) {
            let (a, b) = (mapa.lados[a].sector, mapa.lados[b].sector);
            if a != b {
                vecinos.entry(a).or_default().push(b);
                vecinos.entry(b).or_default().push(a);
            }
        }
    }
    for s in 0..mapa.sectores.len() {
        if mapa.sectores[s].techo <= mapa.sectores[s].suelo {
            let minimo = vecinos.get(&s).into_iter().flatten().map(|v| mapa.sectores[*v].techo).fold(f64::MAX, f64::min);
            if minimo < f64::MAX {
                mapa.sectores[s].techo = minimo - 4.0;
            }
        }
    }
}

/// Una superficie de la geometría original: triángulos con su textura.
pub struct Superficie {
    pub textura: String,
    /// Flat de suelo/techo (64×64) o textura de pared.
    pub es_flat: bool,
    /// Vértices en coordenadas neutrales, de tres en tres.
    pub puntos: Vec<[f32; 3]>,
    /// Coordenadas de textura en metros (32 píxeles de Doom = 1).
    pub uv: Vec<[f32; 2]>,
    pub luz: u8,
}

type P2 = (f64, f64);

/// Recorta un polígono convexo por una recta: se queda con el lado donde
/// `signo · (d × (p − a)) ≤ 0` (el lado "frontal" de Doom con signo = 1).
fn recortar(poli: &[P2], a: P2, d: P2, signo: f64) -> Vec<P2> {
    const EPS: f64 = 1e-6;
    let valor = |p: P2| signo * (d.0 * (p.1 - a.1) - d.1 * (p.0 - a.0));
    let mut fuera = Vec::with_capacity(poli.len() + 2);
    for i in 0..poli.len() {
        let (p, q) = (poli[i], poli[(i + 1) % poli.len()]);
        let (vp, vq) = (valor(p), valor(q));
        if vp <= EPS {
            fuera.push(p);
        }
        if (vp < -EPS && vq > EPS) || (vp > EPS && vq < -EPS) {
            let t = vp / (vp - vq);
            fuera.push((p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t));
        }
    }
    fuera
}

/// La geometría exacta de un mapa (paredes con sus texturas superior, media e
/// inferior, y suelos/techos de cada subsector), para que un visor Doom dibuje
/// el mapa original cuando el mundo viene de Doom. Mismo origen que `cargar`.
pub fn geometria_nativa(nombre_mapa: &str) -> Result<Vec<Superficie>, String> {
    let wad = std::fs::read(Path::new(WAD)).map_err(|e| format!("could not read {WAD}: {e}"))?;
    let mut mapa = leer_mapa(&wad, nombre_mapa)?;
    abrir_puertas(&mut mapa);
    let &(ox, oy, _) = mapa.cosas.iter().find(|c| c.2 == 1).ok_or("el mapa no tiene inicio del jugador 1")?;
    let suelo_origen = mapa.sector_en(ox, oy).map_or(0.0, |s| mapa.sectores[s].suelo);
    let neutral = |x: f64, y: f64, h: f64| [((x - ox) / UNIDADES_POR_CELDA) as f32, ((h - suelo_origen) / UNIDADES_POR_CELDA) as f32, (-(y - oy) / UNIDADES_POR_CELDA) as f32];
    let es_cielo = |s: usize| mapa.sectores[s].flat_techo.starts_with("F_SKY");
    let mut sup = Vec::new();

    // Paredes: cada lado de cada línea, mirando hacia su sector.
    for l in &mapa.lineas {
        for (este, otro, desde, hasta) in [(l.derecha, l.izquierda, l.v1, l.v2), (l.izquierda, l.derecha, l.v2, l.v1)] {
            let Some(lado) = este.map(|s| &mapa.lados[s]) else { continue };
            let f = &mapa.sectores[lado.sector];
            let (a, b) = (mapa.vertices[desde], mapa.vertices[hasta]);
            let largo = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let mut tramos: Vec<(&str, f64, f64)> = Vec::new();
            match otro.map(|s| &mapa.lados[s]) {
                None => tramos.push((&lado.medio, f.suelo, f.techo)),
                Some(atras) => {
                    let b_sec = &mapa.sectores[atras.sector];
                    if f.techo > b_sec.techo && !(es_cielo(lado.sector) && es_cielo(atras.sector)) {
                        tramos.push((&lado.arriba, b_sec.techo, f.techo));
                    }
                    if f.suelo < b_sec.suelo {
                        tramos.push((&lado.abajo, f.suelo, b_sec.suelo));
                    }
                }
            }
            for (textura, abajo, arriba) in tramos {
                if textura.is_empty() || textura == "-" || arriba <= abajo {
                    continue;
                }
                let (u0, u1) = (lado.desplaza_x, lado.desplaza_x + largo);
                let (v0, v1) = (lado.desplaza_y, lado.desplaza_y + (arriba - abajo));
                let esquinas = [neutral(a.0, a.1, arriba), neutral(b.0, b.1, arriba), neutral(b.0, b.1, abajo), neutral(a.0, a.1, abajo)];
                let uvs = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]].map(|[u, v]| [(u / 32.0) as f32, (v / 32.0) as f32]);
                let mut s = Superficie { textura: textura.to_string(), es_flat: false, puntos: Vec::new(), uv: Vec::new(), luz: f.luz };
                for i in [0, 1, 2, 0, 2, 3] {
                    s.puntos.push(esquinas[i]);
                    s.uv.push(uvs[i]);
                }
                sup.push(s);
            }
        }
    }

    // Suelos y techos: cada subsector es un polígono convexo que sale de
    // recortar el plano con las rectas del árbol BSP y con sus propios segs.
    let (min_x, max_x) = mapa.vertices.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v.0), b.max(v.0)));
    let (min_y, max_y) = mapa.vertices.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v.1), b.max(v.1)));
    let caja = vec![(min_x - 64.0, min_y - 64.0), (max_x + 64.0, min_y - 64.0), (max_x + 64.0, max_y + 64.0), (min_x - 64.0, max_y + 64.0)];
    let mut pila = vec![(mapa.nodos.len() as u16 - 1, caja)];
    while let Some((hijo, poli)) = pila.pop() {
        if poli.len() < 3 {
            continue;
        }
        if hijo & 0x8000 == 0 {
            let n = &mapa.nodos[hijo as usize];
            pila.push((n.hijos[0], recortar(&poli, (n.x, n.y), (n.dx, n.dy), 1.0)));
            pila.push((n.hijos[1], recortar(&poli, (n.x, n.y), (n.dx, n.dy), -1.0)));
            continue;
        }
        let ss = (hijo & 0x7FFF) as usize;
        let (primero, cuantos) = (mapa.subsectores[ss], mapa.subsectores_largo[ss]);
        let mut poli = poli;
        for seg in primero..primero + cuantos {
            let (va, vb) = mapa.segs_vertices[seg];
            let (a, b) = (mapa.vertices[va], mapa.vertices[vb]);
            poli = recortar(&poli, a, (b.0 - a.0, b.1 - a.1), 1.0);
        }
        if poli.len() < 3 {
            continue;
        }
        let (linea, sentido) = mapa.segs_linea[primero];
        let l = &mapa.lineas[linea];
        let Some(sector) = (if sentido == 0 { l.derecha } else { l.izquierda }).map(|s| mapa.lados[s].sector) else { continue };
        let sec = &mapa.sectores[sector];
        let mut plano = |textura: &str, h: f64, invertir: bool| {
            let mut s = Superficie { textura: textura.to_string(), es_flat: true, puntos: Vec::new(), uv: Vec::new(), luz: sec.luz };
            for i in 1..poli.len() - 1 {
                let tri = if invertir { [0, i + 1, i] } else { [0, i, i + 1] };
                for k in tri {
                    let (x, y) = poli[k];
                    s.puntos.push(neutral(x, y, h));
                    s.uv.push([(x / 32.0) as f32, (-y / 32.0) as f32]);
                }
            }
            sup.push(s);
        };
        plano(&sec.flat_suelo, sec.suelo, false);
        if !es_cielo(sector) {
            plano(&sec.flat_techo, sec.techo, true);
        }
    }
    Ok(sup)
}

pub fn cargar(nombre_mapa: &str) -> Result<Terreno, String> {
    let wad = std::fs::read(Path::new(WAD)).map_err(|e| format!("could not read {WAD}: {e}"))?;
    let mut mapa = leer_mapa(&wad, nombre_mapa)?;
    abrir_puertas(&mut mapa);

    // Material de pared de cada sector: la textura más usada en sus muros de una cara.
    let mut cuenta: HashMap<(usize, &'static str), usize> = HashMap::new();
    for l in &mapa.lineas {
        if l.izquierda.is_none()
            && let Some(d) = l.derecha
        {
            *cuenta.entry((mapa.lados[d].sector, material_neutral(&mapa.lados[d].medio))).or_default() += 1;
        }
    }
    let mut pared_de: HashMap<usize, (&'static str, usize)> = HashMap::new();
    for ((s, m), c) in cuenta {
        let e = pared_de.entry(s).or_insert((m, 0));
        if c > e.1 {
            *e = (m, c);
        }
    }

    // El jugador 1 es el origen neutral.
    let &(ox, oy, _) = mapa.cosas.iter().find(|c| c.2 == 1).ok_or("el mapa no tiene inicio del jugador 1")?;
    let suelo_origen = mapa.sector_en(ox, oy).map_or(0.0, |s| mapa.sectores[s].suelo);
    let (min_x, max_x) = mapa.vertices.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v.0), b.max(v.0)));
    let (min_y, max_y) = mapa.vertices.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v.1), b.max(v.1)));
    let alcance = [ox - min_x, max_x - ox, oy - min_y, max_y - oy].into_iter().fold(0.0, f64::max);
    let mitad = (alcance / UNIDADES_POR_CELDA).ceil() as usize + MARGEN;
    let lado = mitad * 2;

    let celdas = |u: f64| (u / UNIDADES_POR_CELDA).round() as i16;
    let a_doom = |i: usize, j: usize| {
        let x = ox + (i as f64 - mitad as f64 + 0.5) * UNIDADES_POR_CELDA;
        let y = oy - (j as f64 - mitad as f64 + 0.5) * UNIDADES_POR_CELDA;
        (x, y)
    };

    let mut paleta: Vec<String> = Vec::new();
    let mut indice_material = |m: &str| -> u8 {
        match paleta.iter().position(|p| p == m) {
            Some(i) => i as u8,
            None => {
                paleta.push(m.to_string());
                (paleta.len() - 1) as u8
            }
        }
    };

    let n = lado * lado;
    let mut sector_celda: Vec<Option<usize>> = vec![None; n];
    for j in 0..lado {
        for i in 0..lado {
            let (x, y) = a_doom(i, j);
            sector_celda[j * lado + i] = mapa.sector_en(x, y);
        }
    }

    let (mut alturas, mut techo, mut solido, mut material, mut luz) =
        (vec![0i16; n], vec![SIN_TECHO; n], vec![true; n], vec![0u8; n], vec![0u8; n]);
    let mut cielo = vec![false; n];
    for k in 0..n {
        if let Some(s) = sector_celda[k] {
            let sec = &mapa.sectores[s];
            alturas[k] = celdas(sec.suelo - suelo_origen);
            // El techo con cielo (F_SKY1) conserva su altura: hasta ahí suben las fachadas.
            techo[k] = celdas(sec.techo - suelo_origen);
            cielo[k] = sec.flat_techo.starts_with("F_SKY");
            solido[k] = techo[k] <= alturas[k];
            material[k] = indice_material(material_neutral(&sec.flat_suelo));
            luz[k] = sec.luz;
        }
    }
    // En Doom las paredes son líneas sin grosor: donde dos zonas están separadas
    // por menos de una celda, la rejilla no las ve y quedaría un hueco. Se marca
    // como sólida la celda del lado del vacío de cada pared de una cara, sin
    // tocar celdas con apariciones.
    let celda_de = |x: f64, y: f64| -> Option<usize> {
        let i = ((x - ox) / UNIDADES_POR_CELDA + mitad as f64).floor();
        let j = ((oy - y) / UNIDADES_POR_CELDA + mitad as f64).floor();
        (i >= 0.0 && j >= 0.0 && (i as usize) < lado && (j as usize) < lado).then(|| j as usize * lado + i as usize)
    };
    let protegidas: Vec<usize> = mapa.cosas.iter().filter(|c| c.2 == 11 || (1..=4).contains(&c.2)).filter_map(|c| celda_de(c.0, c.1)).collect();
    let mut selladas = 0;
    let sellar = std::env::var_os("MULTIVERSO_SIN_SELLAR").is_none();
    for l in mapa.lineas.iter().filter(|_| sellar) {
        // Solo paredes de una cara: sellar también las infranqueables de dos caras
        // (barandillas, ventanas) cerraba pasos (de 3018 a 498 celdas alcanzables).
        if l.izquierda.is_some() {
            continue;
        }
        let (x1, y1) = mapa.vertices[l.v1];
        let (x2, y2) = mapa.vertices[l.v2];
        let largo = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt().max(1.0);
        // Normal izquierda: en una pared de una cara, el sector queda a la derecha y el vacío a la izquierda.
        let (nx, ny) = (-(y2 - y1) / largo, (x2 - x1) / largo);
        let pasos = (largo / 4.0).ceil() as usize;
        for p in 0..=pasos {
            let f = p as f64 / pasos as f64;
            let (x, y) = (x1 + (x2 - x1) * f, y1 + (y2 - y1) * f);
            // Se sella la primera celda detrás de la pared cuyo centro queda de
            // verdad del lado del vacío: si el centro cae del lado del sector,
            // sellarla cerraría pasillos estrechos (escaleras de E1M1).
            for distancia in [3.0, 16.0, 29.0] {
                let Some(k) = celda_de(x + nx * distancia, y + ny * distancia) else { break };
                let (cx, cy) = a_doom(k % lado, k / lado);
                let lado_del_centro = (x2 - x1) * (cy - y1) - (y2 - y1) * (cx - x1);
                if lado_del_centro > 0.0 {
                    if !solido[k] && !protegidas.contains(&k) {
                        solido[k] = true;
                        selladas += 1;
                    }
                    break;
                }
            }
        }
    }
    eprintln!("Doom {nombre_mapa}: {selladas} cells sealed behind thin walls");

    // Las celdas sólidas toman la pared de un vecino abierto: su material y su altura de techo.
    let original_solido = solido.clone();
    for j in 0..lado {
        for i in 0..lado {
            let k = j * lado + i;
            if !original_solido[k] {
                continue;
            }
            let mut tope = None;
            for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                let (vi, vj) = (i as i32 + di, j as i32 + dj);
                if vi < 0 || vj < 0 || vi >= lado as i32 || vj >= lado as i32 {
                    continue;
                }
                let v = vj as usize * lado + vi as usize;
                if original_solido[v] {
                    continue;
                }
                let t = techo[v];
                if tope.is_none_or(|(tt, _)| t > tt) {
                    let pared = sector_celda[v].and_then(|s| pared_de.get(&s)).map_or("core:pared", |p| p.0);
                    tope = Some((t, pared));
                }
                luz[k] = luz[k].max(luz[v]);
            }
            match tope {
                Some((t, pared)) => {
                    alturas[k] = t;
                    material[k] = indice_material(pared);
                }
                // Vacío lejos de cualquier sala: no se dibuja (altura mínima).
                None => alturas[k] = i16::MIN / 2,
            }
            techo[k] = alturas[k];
        }
    }

    let suelo_min = (0..n).filter(|k| !solido[*k]).map(|k| alturas[k]).min().unwrap_or(0) - 2;
    for k in 0..n {
        if alturas[k] == i16::MIN / 2 {
            alturas[k] = suelo_min;
            techo[k] = suelo_min;
        }
    }

    // Celdas alcanzables a pie desde el inicio (escalones de 1 m como máximo):
    // las apariciones aisladas (tras ascensores o interruptores) no se usan.
    let inicio = celda_de(ox, oy).unwrap_or(0);
    let mut visto = vec![false; n];
    let mut pila = vec![inicio];
    visto[inicio] = true;
    while let Some(k) = pila.pop() {
        let (i, j) = ((k % lado) as i64, (k / lado) as i64);
        for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (vi, vj) = (i + di, j + dj);
            if vi < 0 || vj < 0 || vi >= lado as i64 || vj >= lado as i64 {
                continue;
            }
            let v = vj as usize * lado + vi as usize;
            if !visto[v] && !solido[v] && alturas[v] - alturas[k] <= 1 {
                visto[v] = true;
                pila.push(v);
            }
        }
    }
    let alcanzables = visto.iter().filter(|v| **v).count();
    let aisladas = protegidas.iter().filter(|k| !visto[**k]).count();
    eprintln!("Doom {nombre_mapa}: {alcanzables} cells reachable from the start, {aisladas} of {} spawn points isolated (unused)", protegidas.len());

    let apariciones = mapa
        .cosas
        .iter()
        .filter(|c| c.2 == 11 || (1..=4).contains(&c.2))
        .filter(|c| celda_de(c.0, c.1).is_some_and(|k| visto[k]))
        .map(|&(x, y, _)| [((x - ox) / UNIDADES_POR_CELDA) as f32, (-(y - oy) / UNIDADES_POR_CELDA) as f32])
        .collect();

    Ok(Terreno {
        origen: [ox as i32, suelo_origen as i32, oy as i32],
        lado,
        alturas,
        agua: vec![false; n],
        nivel_mar: suelo_min - 8,
        fuente: format!("doom:{nombre_mapa}"),
        techo,
        solido,
        material,
        paleta,
        luz,
        apariciones,
        cielo,
    })
}
