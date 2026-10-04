//! Mundos nativos de Signet (`fuente = "signet:<id>"`).
//!
//! Los mundos de otros juegos llegan al terreno neutral a través de un
//! importador, y algo se pierde por el camino. Un mundo nativo está escrito
//! directamente para Signet: además de la geometría tiene luces de colores,
//! objetos y cielo. Como es contenido propio y libre, el servidor lo envía
//! completo en la bienvenida ([`crate::protocolo::DelServidor::Bienvenida`]):
//!
//! * un visor de Signet lo dibuja **nativo**, con todo su detalle;
//! * los demás reciben su proyección a [`Terreno`] y lo **traducen** como
//!   cualquier otro mundo: una columna es un pilar para Doom y bloques para Minecraft.
//!
//! Se escribe a mano en formato de texto `.mvm` (capas en ASCII) o por código
//! con [`ConstructorMapa`]. Ver la documentación, "Mundos nativos".

use serde::{Deserialize, Serialize};

use crate::protocolo::{SIN_TECHO, Terreno};

/// Altura de pared que se usa junto a celdas con cielo (no hay techo que la limite).
const PARED_CIELO: i16 = 6;

/// Luz puntual.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Luz {
    /// Posición en coordenadas neutrales (x, y, z).
    pub pos: [f32; 3],
    pub color: [u8; 3],
    /// Intensidad relativa (1 = una lámpara normal).
    pub intensidad: f32,
    /// Distancia en metros a la que deja de iluminar.
    pub alcance: f32,
}

/// Objeto colocado en el mundo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Objeto {
    /// "columna", "cristal", "farola", "caja", "arco"… Un visor que no conozca
    /// un tipo dibuja una caja.
    pub tipo: String,
    /// Posición de la base en coordenadas neutrales.
    pub pos: [f32; 3],
    pub alto: f32,
    pub color: [u8; 3],
    /// Si bloquea el paso: su celda pasa a ser pared en el terreno neutral.
    pub solido: bool,
}

/// Un mundo nativo de Signet.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Mapa {
    /// Identificador (lo que va tras `signet:`).
    pub id: String,
    pub nombre: String,
    pub autor: String,
    pub licencia: String,
    pub lado: usize,
    /// Altura del suelo de cada celda (fila a fila, z y luego x).
    pub suelo: Vec<i16>,
    /// Celdas de pared.
    pub pared: Vec<bool>,
    /// Altura del techo; `None` = cielo abierto.
    pub techo: Vec<Option<i16>>,
    /// Material de cada celda (suelo, o pared si es pared): índice en `paleta`.
    pub material: Vec<u8>,
    pub paleta: Vec<String>,
    pub luces: Vec<Luz>,
    pub objetos: Vec<Objeto>,
    /// Puntos de aparición (x, z) neutrales.
    pub apariciones: Vec<[f32; 2]>,
    /// Color del cielo arriba y en el horizonte.
    pub cielo: [[u8; 3]; 2],
    /// Luz ambiente 0–1.
    pub ambiente: f32,
}

impl Mapa {
    /// `fuente` del terreno neutral que sale de este mapa.
    pub fn fuente(&self) -> String {
        format!("signet:{}", self.id)
    }

    /// Centro neutral de la celda (i, j).
    pub fn centro(&self, i: usize, j: usize) -> (f32, f32) {
        let mitad = (self.lado / 2) as f32;
        (i as f32 - mitad + 0.5, j as f32 - mitad + 0.5)
    }

    /// Celda de una posición neutral, si cae dentro del mapa.
    pub fn celda(&self, x: f32, z: f32) -> Option<(usize, usize)> {
        let mitad = (self.lado / 2) as f32;
        let (i, j) = ((x + mitad).floor(), (z + mitad).floor());
        (i >= 0.0 && j >= 0.0 && (i as usize) < self.lado && (j as usize) < self.lado).then_some((i as usize, j as usize))
    }

    /// Proyección al terreno neutral: lo que reciben los traductores de otros juegos.
    pub fn a_terreno(&self) -> Terreno {
        let lado = self.lado;
        let n = lado * lado;
        let mut pared = self.pared.clone();
        let mut alturas = self.suelo.clone();
        // Objetos sólidos: su celda es pared hasta lo alto del objeto.
        let mut tope_objeto = vec![None::<i16>; n];
        for o in self.objetos.iter().filter(|o| o.solido) {
            if let Some((i, j)) = self.celda(o.pos[0], o.pos[2]) {
                let k = j * lado + i;
                pared[k] = true;
                tope_objeto[k] = Some((o.pos[1] + o.alto).ceil() as i16);
            }
        }
        let techo_de = |k: usize| self.techo[k].unwrap_or(self.suelo[k] + PARED_CIELO);
        for j in 0..lado {
            for i in 0..lado {
                let k = j * lado + i;
                if !pared[k] {
                    continue;
                }
                // Una pared sube hasta el techo más alto de sus vecinas transitables, como en los importadores.
                let mut tope = tope_objeto[k];
                for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                    let (vi, vj) = (i as i32 + di, j as i32 + dj);
                    if vi < 0 || vj < 0 || vi as usize >= lado || vj as usize >= lado {
                        continue;
                    }
                    let v = vj as usize * lado + vi as usize;
                    if !pared[v] && tope_objeto[k].is_none() {
                        tope = Some(tope.unwrap_or(i16::MIN).max(techo_de(v)));
                    }
                }
                alturas[k] = tope.unwrap_or(self.suelo[k]);
            }
        }
        let techo: Vec<i16> = (0..n)
            .map(|k| if pared[k] { alturas[k] } else { self.techo[k].unwrap_or(SIN_TECHO) })
            .collect();
        let cielo: Vec<bool> = (0..n).map(|k| !pared[k] && self.techo[k].is_none()).collect();
        let agua = self.material.iter().map(|m| self.paleta.get(*m as usize).is_some_and(|p| p == "core:agua")).collect();
        // Luz horneada por celda para los visores que no tienen luces dinámicas.
        let luz = (0..n)
            .map(|k| {
                let (x, z) = self.centro(k % lado, k / lado);
                let y = self.suelo[k] as f32 + 1.0;
                let mut l = self.ambiente;
                if self.techo[k].is_none() {
                    l += 0.25;
                }
                for f in &self.luces {
                    let d = ((f.pos[0] - x).powi(2) + (f.pos[1] - y).powi(2) + (f.pos[2] - z).powi(2)).sqrt();
                    if d < f.alcance {
                        l += f.intensidad * (1.0 - d / f.alcance).powi(2) * 0.8;
                    }
                }
                (l.clamp(0.0, 1.0) * 255.0) as u8
            })
            .collect();
        let suelo_min = (0..n).filter(|k| !pared[*k]).map(|k| self.suelo[k]).min().unwrap_or(0);
        Terreno {
            origen: [0, 0, 0],
            lado,
            alturas,
            agua,
            nivel_mar: suelo_min - 10,
            fuente: self.fuente(),
            techo,
            solido: pared,
            material: self.material.clone(),
            paleta: self.paleta.clone(),
            luz,
            apariciones: self.apariciones.clone(),
            cielo,
        }
    }

    /// Lee un mapa en formato de texto `.mvm`.
    pub fn desde_mvm(texto: &str) -> Result<Mapa, String> {
        leer_mvm(texto)
    }

    /// Escribe el mapa en formato de texto `.mvm`.
    pub fn a_mvm(&self) -> String {
        escribir_mvm(self)
    }
}

fn color_de(p: &[&str]) -> Result<[u8; 3], String> {
    if p.len() < 3 {
        return Err("color: missing r g b components".into());
    }
    let c = |s: &str| s.parse::<u8>().map_err(|_| format!("invalid colour component: {s}"));
    Ok([c(p[0])?, c(p[1])?, c(p[2])?])
}

fn numero(s: &str) -> Result<f32, String> {
    s.parse::<f32>().map_err(|_| format!("invalid number: {s}"))
}

/// Altura codificada en un carácter: '0'–'9' y 'a'–'z' (10–35).
fn altura_de(c: char) -> Option<i16> {
    match c {
        '0'..='9' => Some(c as i16 - '0' as i16),
        'a'..='z' => Some(c as i16 - 'a' as i16 + 10),
        _ => None,
    }
}

fn caracter_de(h: i16) -> char {
    match h {
        0..=9 => (b'0' + h as u8) as char,
        10..=35 => (b'a' + (h - 10) as u8) as char,
        _ => '0',
    }
}

fn leer_mvm(texto: &str) -> Result<Mapa, String> {
    let mut m = Mapa {
        id: String::new(),
        nombre: String::new(),
        autor: String::new(),
        licencia: String::new(),
        lado: 0,
        suelo: Vec::new(),
        pared: Vec::new(),
        techo: Vec::new(),
        material: Vec::new(),
        paleta: Vec::new(),
        luces: Vec::new(),
        objetos: Vec::new(),
        apariciones: Vec::new(),
        cielo: [[30, 34, 60], [110, 90, 200]],
        ambiente: 0.3,
    };
    let mut leyenda: Vec<(char, String)> = Vec::new();
    let mut capas: std::collections::HashMap<String, Vec<String>> = Default::default();
    let mut capa: Option<(String, Vec<String>)> = None;
    // Las posiciones del archivo están en celdas desde la esquina noroeste; se pasan a neutrales al final.
    let mut luces_celda: Vec<(f32, f32, f32, [u8; 3], f32, f32)> = Vec::new();
    let mut objetos_celda: Vec<(String, f32, f32, f32, [u8; 3], bool)> = Vec::new();
    let mut apariciones_celda: Vec<(f32, f32)> = Vec::new();
    for (num, linea) in texto.lines().enumerate() {
        let err = |e: String| format!("line {}: {e}", num + 1);
        if let Some((nombre, filas)) = capa.as_mut() {
            if linea.trim() == "fin" {
                let (nombre, filas) = capa.take().unwrap();
                capas.insert(nombre, filas);
            } else {
                let _ = nombre;
                filas.push(linea.trim_end().to_string());
            }
            continue;
        }
        let linea = linea.split('#').next().unwrap_or("").trim();
        if linea.is_empty() {
            continue;
        }
        let p: Vec<&str> = linea.split_whitespace().collect();
        match p[0] {
            "mapa" => m.id = p.get(1).ok_or_else(|| err("missing id".into()))?.to_string(),
            "nombre" => m.nombre = p[1..].join(" "),
            "autor" => m.autor = p[1..].join(" "),
            "licencia" => m.licencia = p[1..].join(" "),
            "lado" => m.lado = p.get(1).and_then(|s| s.parse().ok()).ok_or_else(|| err("invalid side".into()))?,
            "cielo" => {
                m.cielo = [color_de(&p[1..]).map_err(err)?, color_de(p.get(4..).unwrap_or(&[])).map_err(err)?];
            }
            "ambiente" => m.ambiente = numero(p.get(1).unwrap_or(&"")).map_err(err)?,
            "leyenda" => {
                let c = p.get(1).and_then(|s| s.chars().next()).ok_or_else(|| err("legend without a character".into()))?;
                let nombre = p.get(2).ok_or_else(|| err("legend without a material".into()))?;
                leyenda.push((c, nombre.to_string()));
            }
            "suelo" | "techo" | "material" => capa = Some((p[0].to_string(), Vec::new())),
            "luz" => {
                if p.len() < 9 {
                    return Err(err("usage: luz x z y r g b intensity range".into()));
                }
                luces_celda.push((numero(p[1]).map_err(err)?, numero(p[2]).map_err(err)?, numero(p[3]).map_err(err)?, color_de(&p[4..7]).map_err(err)?, numero(p[7]).map_err(err)?, numero(p[8]).map_err(err)?));
            }
            "objeto" => {
                if p.len() < 8 {
                    return Err(err("usage: objeto type x z height r g b [solido]".into()));
                }
                objetos_celda.push((p[1].to_string(), numero(p[2]).map_err(err)?, numero(p[3]).map_err(err)?, numero(p[4]).map_err(err)?, color_de(&p[5..8]).map_err(err)?, p.get(8) == Some(&"solido")));
            }
            "aparicion" => {
                if p.len() < 3 {
                    return Err(err("usage: aparicion x z".into()));
                }
                apariciones_celda.push((numero(p[1]).map_err(err)?, numero(p[2]).map_err(err)?));
            }
            otro => return Err(err(format!("unknown instruction: {otro}"))),
        }
    }
    if capa.is_some() {
        return Err("a layer does not end with «fin»".into());
    }
    if m.id.is_empty() || m.lado == 0 {
        return Err("missing «mapa <id>» or «lado <n>»".into());
    }
    let lado = m.lado;
    let n = lado * lado;
    let filas = |nombre: &str| -> Result<Vec<Vec<char>>, String> {
        let f = capas.get(nombre).ok_or(format!("missing layer «{nombre}»"))?;
        if f.len() != lado {
            return Err(format!("layer «{nombre}» has {} rows and must have {lado}", f.len()));
        }
        f.iter()
            .enumerate()
            .map(|(j, fila)| {
                let c: Vec<char> = fila.chars().collect();
                if c.len() != lado {
                    Err(format!("layer «{nombre}», row {}: {} characters, must be {lado}", j + 1, c.len()))
                } else {
                    Ok(c)
                }
            })
            .collect()
    };
    let suelo = filas("suelo")?;
    let techo = filas("techo")?;
    let material = filas("material")?;
    m.paleta = leyenda.iter().map(|(_, n)| n.clone()).collect();
    if m.paleta.is_empty() {
        m.paleta.push("core:piedra".into());
    }
    m.suelo = vec![0; n];
    m.pared = vec![false; n];
    m.techo = vec![None; n];
    m.material = vec![0; n];
    for j in 0..lado {
        for i in 0..lado {
            let k = j * lado + i;
            match suelo[j][i] {
                '#' => m.pared[k] = true,
                c => m.suelo[k] = altura_de(c).ok_or(format!("layer «suelo», row {} column {}: invalid character «{c}»", j + 1, i + 1))?,
            }
            m.techo[k] = match techo[j][i] {
                '.' | '#' => None,
                c => Some(altura_de(c).ok_or(format!("layer «techo», row {} column {}: invalid character «{c}»", j + 1, i + 1))?),
            };
            let c = material[j][i];
            m.material[k] = leyenda.iter().position(|(l, _)| *l == c).unwrap_or(0) as u8;
        }
    }
    let mitad = (lado / 2) as f32;
    let suelo_en = |m: &Mapa, x: f32, z: f32| m.celda(x - mitad, z - mitad).map_or(0.0, |(i, j)| m.suelo[j * lado + i] as f32);
    m.luces = luces_celda
        .into_iter()
        .map(|(x, z, y, color, intensidad, alcance)| Luz { pos: [x - mitad, suelo_en(&m, x, z) + y, z - mitad], color, intensidad, alcance })
        .collect();
    m.objetos = objetos_celda
        .into_iter()
        .map(|(tipo, x, z, alto, color, solido)| Objeto { pos: [x - mitad, suelo_en(&m, x, z), z - mitad], tipo, alto, color, solido })
        .collect();
    m.apariciones = apariciones_celda.into_iter().map(|(x, z)| [x - mitad, z - mitad]).collect();
    Ok(m)
}

fn escribir_mvm(m: &Mapa) -> String {
    let lado = m.lado;
    let mitad = (lado / 2) as f32;
    let letras: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
    let mut s = String::new();
    s += &format!("# Native Signet world (.mvm format)\nmapa {}\nnombre {}\nautor {}\nlicencia {}\nlado {lado}\n", m.id, m.nombre, m.autor, m.licencia);
    s += &format!("cielo {} {} {}  {} {} {}\nambiente {}\n\n", m.cielo[0][0], m.cielo[0][1], m.cielo[0][2], m.cielo[1][0], m.cielo[1][1], m.cielo[1][2], m.ambiente);
    for (k, p) in m.paleta.iter().enumerate() {
        s += &format!("leyenda {} {p}\n", letras[k % letras.len()]);
    }
    let capa = |nombre: &str, f: &dyn Fn(usize) -> char| {
        let mut c = format!("\n{nombre}\n");
        for j in 0..lado {
            c.extend((0..lado).map(|i| f(j * lado + i)));
            c.push('\n');
        }
        c + "fin\n"
    };
    s += &capa("suelo", &|k| if m.pared[k] { '#' } else { caracter_de(m.suelo[k]) });
    s += &capa("techo", &|k| if m.pared[k] { '#' } else { m.techo[k].map_or('.', caracter_de) });
    s += &capa("material", &|k| letras[m.material[k] as usize % letras.len()]);
    s += "\n";
    let suelo_en = |x: f32, z: f32| m.celda(x, z).map_or(0.0, |(i, j)| m.suelo[j * lado + i] as f32);
    // Centésimas: evita escribir 3.8000002 por el redondeo de f32.
    let r = |v: f32| (v * 100.0).round() / 100.0;
    for l in &m.luces {
        s += &format!("luz {} {} {} {} {} {} {} {}\n", r(l.pos[0] + mitad), r(l.pos[2] + mitad), r(l.pos[1] - suelo_en(l.pos[0], l.pos[2])), l.color[0], l.color[1], l.color[2], r(l.intensidad), r(l.alcance));
    }
    for o in &m.objetos {
        s += &format!("objeto {} {} {} {} {} {} {}{}\n", o.tipo, r(o.pos[0] + mitad), r(o.pos[2] + mitad), r(o.alto), o.color[0], o.color[1], o.color[2], if o.solido { " solido" } else { "" });
    }
    for a in &m.apariciones {
        s += &format!("aparicion {} {}\n", r(a[0] + mitad), r(a[1] + mitad));
    }
    s
}

/// Construye un mapa por código: salas, escaleras, objetos y luces.
///
/// ```
/// use signet_sdk::mundo::ConstructorMapa;
/// let mut c = ConstructorMapa::nuevo("prueba", "Prueba", 16);
/// let metal = c.material("core:metal");
/// c.sala(2, 2, 13, 13, 0, Some(4), metal);
/// c.aparicion(4.5, 4.5);
/// let mapa = c.construir();
/// assert_eq!(mapa.a_terreno().fuente, "signet:prueba");
/// ```
pub struct ConstructorMapa {
    m: Mapa,
}

impl ConstructorMapa {
    /// Un mapa de `lado × lado` celdas, todo pared, para ir vaciando salas.
    pub fn nuevo(id: &str, nombre: &str, lado: usize) -> Self {
        let n = lado * lado;
        Self {
            m: Mapa {
                id: id.into(),
                nombre: nombre.into(),
                autor: "Signet Protocol".into(),
                licencia: "CC0-1.0".into(),
                lado,
                suelo: vec![0; n],
                pared: vec![true; n],
                techo: vec![None; n],
                material: vec![0; n],
                paleta: vec!["core:pared".into()],
                luces: Vec::new(),
                objetos: Vec::new(),
                apariciones: Vec::new(),
                cielo: [[30, 34, 60], [110, 90, 200]],
                ambiente: 0.3,
            },
        }
    }

    pub fn autor(&mut self, autor: &str, licencia: &str) -> &mut Self {
        self.m.autor = autor.into();
        self.m.licencia = licencia.into();
        self
    }

    pub fn cielo(&mut self, arriba: [u8; 3], horizonte: [u8; 3], ambiente: f32) -> &mut Self {
        self.m.cielo = [arriba, horizonte];
        self.m.ambiente = ambiente;
        self
    }

    /// Índice de un material de la paleta (lo añade si no está).
    pub fn material(&mut self, nombre: &str) -> u8 {
        if let Some(k) = self.m.paleta.iter().position(|p| p == nombre) {
            return k as u8;
        }
        self.m.paleta.push(nombre.into());
        (self.m.paleta.len() - 1) as u8
    }

    fn cada(&mut self, i0: usize, j0: usize, i1: usize, j1: usize, mut f: impl FnMut(&mut Mapa, usize, usize, usize)) {
        for j in j0.min(j1)..=j0.max(j1).min(self.m.lado - 1) {
            for i in i0.min(i1)..=i0.max(i1).min(self.m.lado - 1) {
                let k = j * self.m.lado + i;
                f(&mut self.m, k, i, j);
            }
        }
    }

    /// Vacía un rectángulo de celdas (incluidos los bordes) con suelo, techo (`None` = cielo) y material.
    pub fn sala(&mut self, i0: usize, j0: usize, i1: usize, j1: usize, suelo: i16, techo: Option<i16>, material: u8) -> &mut Self {
        self.cada(i0, j0, i1, j1, |m, k, _, _| {
            m.pared[k] = false;
            m.suelo[k] = suelo;
            m.techo[k] = techo;
            m.material[k] = material;
        });
        self
    }

    /// Cambia solo el material del suelo (o de la pared) de un rectángulo.
    pub fn pintar(&mut self, i0: usize, j0: usize, i1: usize, j1: usize, material: u8) -> &mut Self {
        self.cada(i0, j0, i1, j1, |m, k, _, _| m.material[k] = material);
        self
    }

    /// Rellena un rectángulo de pared con un material.
    pub fn pared(&mut self, i0: usize, j0: usize, i1: usize, j1: usize, material: u8) -> &mut Self {
        self.cada(i0, j0, i1, j1, |m, k, _, _| {
            m.pared[k] = true;
            m.material[k] = material;
        });
        self
    }

    /// Escalera de peldaños de 1 m: el suelo sube de `desde` a `hasta` a lo largo
    /// del eje x (si `en_x`) o z, empezando por el lado de `i0`/`j0`.
    #[allow(clippy::too_many_arguments)]
    pub fn escalera(&mut self, i0: usize, j0: usize, i1: usize, j1: usize, desde: i16, hasta: i16, en_x: bool, techo: Option<i16>, material: u8) -> &mut Self {
        let (a, b) = if en_x { (i0, i1) } else { (j0, j1) };
        let largo = (b as i32 - a as i32).abs().max(1) as f32;
        self.cada(i0, j0, i1, j1, |m, k, i, j| {
            let pos = if en_x { i } else { j };
            let t = (pos as i32 - a as i32).abs() as f32 / largo;
            m.pared[k] = false;
            m.suelo[k] = desde + ((hasta - desde) as f32 * t).round() as i16;
            m.techo[k] = techo;
            m.material[k] = material;
        });
        self
    }

    /// Luz puntual sobre la celda (x, z), a `y` metros sobre su suelo.
    pub fn luz(&mut self, x: f32, z: f32, y: f32, color: [u8; 3], intensidad: f32, alcance: f32) -> &mut Self {
        let (nx, nz) = self.neutral(x, z);
        let base = self.suelo_en(nx, nz);
        self.m.luces.push(Luz { pos: [nx, base + y, nz], color, intensidad, alcance });
        self
    }

    /// Objeto con su base sobre el suelo de la celda (x, z).
    pub fn objeto(&mut self, tipo: &str, x: f32, z: f32, alto: f32, color: [u8; 3], solido: bool) -> &mut Self {
        let (nx, nz) = self.neutral(x, z);
        let base = self.suelo_en(nx, nz);
        self.m.objetos.push(Objeto { tipo: tipo.into(), pos: [nx, base, nz], alto, color, solido });
        self
    }

    pub fn aparicion(&mut self, x: f32, z: f32) -> &mut Self {
        let (nx, nz) = self.neutral(x, z);
        self.m.apariciones.push([nx, nz]);
        self
    }

    fn neutral(&self, x: f32, z: f32) -> (f32, f32) {
        let mitad = (self.m.lado / 2) as f32;
        (x - mitad, z - mitad)
    }

    fn suelo_en(&self, x: f32, z: f32) -> f32 {
        self.m.celda(x, z).map_or(0.0, |(i, j)| self.m.suelo[j * self.m.lado + i] as f32)
    }

    pub fn construir(&self) -> Mapa {
        self.m.clone()
    }
}

/// Mundos nativos incluidos en el SDK (cualquier servidor y cliente los tiene).
pub const INCLUIDOS: &[(&str, &str)] = &[("nexo", include_str!("../mundos/nexo.mvm"))];

/// Un mundo incluido por su id ("nexo").
pub fn incluido(id: &str) -> Option<Mapa> {
    INCLUIDOS.iter().find(|(i, _)| *i == id).and_then(|(_, t)| Mapa::desde_mvm(t).ok())
}

/// El mapa nativo de un terreno, si su `fuente` es un mundo incluido.
pub fn mapa_nativo(fuente: &str) -> Option<Mapa> {
    // "multiverso:" es el prefijo de las primeras betas; se sigue aceptando.
    fuente.strip_prefix("signet:").or_else(|| fuente.strip_prefix("multiverso:")).and_then(incluido)
}
