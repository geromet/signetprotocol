//! Lectura de los datos de OpenArena (GPL): paquetes `.pk3`, texturas TGA/JPG
//! y modelos MD3 de jugador (piernas + torso + cabeza unidos por sus tags),
//! en la pose de una animación de `animation.cfg`.
//!
//! Escala: 1 unidad de Quake III ≈ 1 pulgada; como en Doom, 32 unidades = 1 m.
//! Ejes del modelo: x de Quake (adelante) → -z neutral, y (izquierda) → -x, z (arriba) → y.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

pub const CARPETA: &str = "datos/openarena/openarena-0.8.8/baseoa";
const UNIDADES_POR_METRO: f32 = 32.0;

/// Todos los `.pk3` de `baseoa`: los posteriores (por nombre) sustituyen a los anteriores.
pub struct Paquetes {
    archivos: HashMap<String, (usize, usize)>,
    zips: Vec<zip::ZipArchive<std::fs::File>>,
}

impl Paquetes {
    pub fn abrir() -> Result<Self, String> {
        let mut nombres: Vec<_> = std::fs::read_dir(CARPETA)
            .map_err(|e| format!("OpenArena not found in {CARPETA}: {e}"))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pk3")))
            .collect();
        nombres.sort();
        let mut zips = Vec::new();
        let mut archivos = HashMap::new();
        for (z, ruta) in nombres.iter().enumerate() {
            let mut zip = zip::ZipArchive::new(std::fs::File::open(ruta).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            for i in 0..zip.len() {
                if let Ok(e) = zip.by_index_raw(i) {
                    archivos.insert(e.name().to_ascii_lowercase(), (z, i));
                }
            }
            zips.push(zip);
        }
        Ok(Self { archivos, zips })
    }

    pub fn leer(&mut self, ruta: &str) -> Option<Vec<u8>> {
        let &(z, i) = self.archivos.get(&ruta.to_ascii_lowercase())?;
        let mut e = self.zips[z].by_index(i).ok()?;
        let mut v = Vec::with_capacity(e.size() as usize);
        e.read_to_end(&mut v).ok()?;
        Some(v)
    }

    /// Bytes de una textura y su extensión. Como el motor, prueba `.tga`, `.jpg`
    /// y `.png` aunque la ruta diga otra extensión (o ninguna).
    pub fn textura(&mut self, ruta: &str) -> Option<(Vec<u8>, &'static str)> {
        let base = Path::new(ruta).with_extension("");
        let base = base.to_string_lossy().replace('\\', "/");
        ["tga", "jpg", "png"].into_iter().find_map(|ext| self.leer(&format!("{base}.{ext}")).map(|b| (b, ext)))
    }
}

fn i32le(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn f32le(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn i16le(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}
fn nombre(b: &[u8], o: usize, n: usize) -> String {
    b[o..o + n].iter().take_while(|c| **c != 0).map(|c| *c as char).collect::<String>().to_ascii_lowercase()
}

/// Un tag de MD3: punto de unión con su orientación (filas = ejes x, y, z).
#[derive(Clone, Copy)]
struct Tag {
    origen: [f32; 3],
    ejes: [[f32; 3]; 3],
}

impl Tag {
    fn aplicar(&self, p: [f32; 3]) -> [f32; 3] {
        let e = &self.ejes;
        [
            self.origen[0] + e[0][0] * p[0] + e[1][0] * p[1] + e[2][0] * p[2],
            self.origen[1] + e[0][1] * p[0] + e[1][1] * p[1] + e[2][1] * p[2],
            self.origen[2] + e[0][2] * p[0] + e[1][2] * p[1] + e[2][2] * p[2],
        ]
    }

    /// Este tag dentro del espacio de `padre` (para encadenar torso → cabeza).
    fn dentro_de(&self, padre: &Tag) -> Tag {
        let rotar = |v: [f32; 3]| {
            let e = &padre.ejes;
            [e[0][0] * v[0] + e[1][0] * v[1] + e[2][0] * v[2], e[0][1] * v[0] + e[1][1] * v[1] + e[2][1] * v[2], e[0][2] * v[0] + e[1][2] * v[1] + e[2][2] * v[2]]
        };
        Tag { origen: padre.aplicar(self.origen), ejes: self.ejes.map(rotar) }
    }
}

struct Superficie {
    nombre: String,
    shader: String,
    triangulos: Vec<[u32; 3]>,
    uv: Vec<[f32; 2]>,
    /// Vértices de cada fotograma.
    fotogramas: Vec<Vec<[f32; 3]>>,
}

struct Md3 {
    tags: Vec<Vec<(String, Tag)>>,
    superficies: Vec<Superficie>,
}

fn leer_md3(b: &[u8]) -> Option<Md3> {
    if b.len() < 108 || &b[0..4] != b"IDP3" {
        return None;
    }
    let (n_frames, n_tags, n_surf) = (i32le(b, 76) as usize, i32le(b, 80) as usize, i32le(b, 84) as usize);
    let (ofs_tags, ofs_surf) = (i32le(b, 96) as usize, i32le(b, 100) as usize);
    let mut tags = Vec::with_capacity(n_frames);
    for f in 0..n_frames {
        let mut fila = Vec::with_capacity(n_tags);
        for t in 0..n_tags {
            let o = ofs_tags + (f * n_tags + t) * 112;
            let v = |k: usize| f32le(b, o + 64 + k * 4);
            // Como `R_LerpTag` del motor, los ejes se normalizan: algunos modelos
            // (Sergei, Skelebot) los guardan escalados.
            let eje = |a: usize| {
                let e = [v(a), v(a + 1), v(a + 2)];
                let l = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt().max(1e-6);
                [e[0] / l, e[1] / l, e[2] / l]
            };
            fila.push((nombre(b, o, 64), Tag { origen: [v(0), v(1), v(2)], ejes: [eje(3), eje(6), eje(9)] }));
        }
        tags.push(fila);
    }
    let mut superficies = Vec::with_capacity(n_surf);
    let mut o = ofs_surf;
    for _ in 0..n_surf {
        let s = o;
        let (frames, n_shaders, n_verts, n_tris) = (i32le(b, s + 72) as usize, i32le(b, s + 76) as usize, i32le(b, s + 80) as usize, i32le(b, s + 84) as usize);
        let (ofs_tri, ofs_sh, ofs_st, ofs_xyz, ofs_fin) = (i32le(b, s + 88) as usize, i32le(b, s + 92) as usize, i32le(b, s + 96) as usize, i32le(b, s + 100) as usize, i32le(b, s + 104) as usize);
        let triangulos = (0..n_tris).map(|i| {
            let t = s + ofs_tri + i * 12;
            [i32le(b, t) as u32, i32le(b, t + 4) as u32, i32le(b, t + 8) as u32]
        });
        let uv = (0..n_verts).map(|i| [f32le(b, s + ofs_st + i * 8), f32le(b, s + ofs_st + i * 8 + 4)]).collect();
        let fotogramas = (0..frames)
            .map(|f| {
                (0..n_verts)
                    .map(|i| {
                        let p = s + ofs_xyz + (f * n_verts + i) * 8;
                        [i16le(b, p) as f32 / 64.0, i16le(b, p + 2) as f32 / 64.0, i16le(b, p + 4) as f32 / 64.0]
                    })
                    .collect()
            })
            .collect();
        superficies.push(Superficie {
            nombre: nombre(b, s + 4, 64),
            shader: if n_shaders > 0 { nombre(b, s + ofs_sh, 64) } else { String::new() },
            triangulos: triangulos.collect(),
            uv,
            fotogramas,
        });
        o = s + ofs_fin;
    }
    Some(Md3 { tags, superficies })
}

/// `superficie,textura` de un archivo `.skin`.
fn leer_skin(texto: &str) -> HashMap<String, String> {
    texto
        .lines()
        .filter_map(|l| l.split_once(','))
        .filter(|(_, t)| !t.trim().is_empty())
        .map(|(s, t)| (s.trim().to_ascii_lowercase(), t.trim().to_ascii_lowercase()))
        .collect()
}

/// Primer fotograma de cada animación de `animation.cfg`, en el orden de Quake III.
fn leer_animaciones(texto: &str) -> Vec<usize> {
    texto
        .lines()
        .filter_map(|l| l.split_whitespace().next()?.parse::<usize>().ok())
        .collect()
}

/// Una pieza de malla lista para dibujar: textura, posiciones neutrales y coordenadas.
pub struct Pieza {
    pub textura: String,
    pub posiciones: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Copy)]
pub enum Pose {
    /// LEGS_IDLE + TORSO_STAND.
    DePie,
    /// BOTH_DEAD1: tumbado tras morir.
    Muerto,
}

/// Modelo de jugador (`models/players/<nombre>/`) en una pose, con los pies en y = 0
/// y mirando hacia -z (el "adelante" neutral con yaw 0).
pub fn modelo_jugador(p: &mut Paquetes, nombre_modelo: &str, pose: Pose) -> Option<Vec<Pieza>> {
    let dir = format!("models/players/{nombre_modelo}");
    let piernas = leer_md3(&p.leer(&format!("{dir}/lower.md3"))?)?;
    let torso = leer_md3(&p.leer(&format!("{dir}/upper.md3"))?)?;
    let cabeza = leer_md3(&p.leer(&format!("{dir}/head.md3"))?)?;
    let anim = p.leer(&format!("{dir}/animation.cfg")).map(|b| leer_animaciones(&String::from_utf8_lossy(&b))).unwrap_or_default();
    // Índices de animación de Quake III: 1 BOTH_DEAD1, 6 TORSO_GESTURE, 11 TORSO_STAND, 13 LEGS_WALKCR, 22 LEGS_IDLE.
    let a = |i: usize| anim.get(i).copied().unwrap_or(0);
    // Las animaciones de piernas cuentan fotogramas después de las del torso.
    let salto = a(13).saturating_sub(a(6));
    let (f_piernas, f_torso) = match pose {
        Pose::DePie => (a(22).saturating_sub(salto), a(11)),
        Pose::Muerto => (a(1), a(1)),
    };
    let skin = |parte: &str, p: &mut Paquetes| p.leer(&format!("{dir}/{parte}_default.skin")).map(|b| leer_skin(&String::from_utf8_lossy(&b))).unwrap_or_default();
    let (skin_p, skin_t, skin_c) = (skin("lower", p), skin("upper", p), skin("head", p));

    let tag = |m: &Md3, f: usize, n: &str| m.tags.get(f.min(m.tags.len().saturating_sub(1))).and_then(|fila| fila.iter().find(|(k, _)| k == n)).map(|(_, t)| *t);
    let identidad = Tag { origen: [0.0; 3], ejes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] };
    let tag_torso = tag(&piernas, f_piernas, "tag_torso").unwrap_or(identidad);
    let tag_cabeza = tag(&torso, f_torso, "tag_head").map(|t| t.dentro_de(&tag_torso)).unwrap_or(tag_torso);

    // El origen de las piernas está en la cadera: los pies quedan 24 unidades por debajo.
    let a_neutral = |q: [f32; 3]| [-q[1] / UNIDADES_POR_METRO, (q[2] + 24.0) / UNIDADES_POR_METRO, -q[0] / UNIDADES_POR_METRO];
    let mut piezas = Vec::new();
    for (m, f, transf, skin) in [(&piernas, f_piernas, identidad, &skin_p), (&torso, f_torso, tag_torso, &skin_t), (&cabeza, 0, tag_cabeza, &skin_c)] {
        for s in &m.superficies {
            let fotograma = &s.fotogramas[f.min(s.fotogramas.len().saturating_sub(1))];
            let textura = skin.get(&s.nombre).cloned().unwrap_or_else(|| s.shader.clone());
            if textura.is_empty() || textura.contains("nodraw") {
                continue;
            }
            piezas.push(Pieza {
                textura,
                posiciones: fotograma.iter().map(|v| a_neutral(transf.aplicar(*v))).collect(),
                uv: s.uv.clone(),
                indices: s.triangulos.iter().flat_map(|t| [t[0], t[2], t[1]]).collect(),
            });
        }
    }
    Some(piezas)
}
