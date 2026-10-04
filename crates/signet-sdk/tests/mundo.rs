//! Mundos nativos: el formato .mvm, su proyección a terreno y Nexo.

use signet_sdk::conformidad::{comprobar_reglas, comprobar_terreno};
use signet_sdk::mundo::{self, ConstructorMapa, Mapa};
use signet_sdk::protocolo::{DelServidor, SIN_TECHO};

#[test]
fn nexo_esta_incluido_y_es_conforme() {
    let m = mundo::incluido("nexo").expect("nexo.mvm se lee");
    assert_eq!(m.lado, 40);
    assert_eq!(m.apariciones.len(), 8);
    let t = m.a_terreno();
    assert_eq!(t.fuente, "signet:nexo");
    assert!(comprobar_terreno(&t).is_empty(), "{:?}", comprobar_terreno(&t));
    assert!(comprobar_reglas(&t).is_empty(), "{:?}", comprobar_reglas(&t));
    assert!(mundo::mapa_nativo("signet:nexo").is_some());
    assert!(mundo::mapa_nativo("multiverso:nexo").is_some(), "prefijo antiguo");
    assert!(mundo::mapa_nativo("doom:E1M1").is_none());
}

#[test]
fn ida_y_vuelta_por_mvm() {
    let m = mundo::incluido("nexo").unwrap();
    let otra = Mapa::desde_mvm(&m.a_mvm()).unwrap();
    assert_eq!(otra.suelo, m.suelo);
    assert_eq!(otra.pared, m.pared);
    assert_eq!(otra.techo, m.techo);
    assert_eq!(otra.material, m.material);
    assert_eq!(otra.objetos.len(), m.objetos.len());
    for (a, b) in otra.luces.iter().zip(&m.luces) {
        assert!((a.pos[1] - b.pos[1]).abs() < 0.01);
    }
}

#[test]
fn objetos_solidos_se_traducen_como_pared() {
    let mut c = ConstructorMapa::nuevo("prueba", "Prueba", 12);
    let piedra = c.material("core:piedra");
    c.sala(1, 1, 10, 10, 0, None, piedra);
    c.objeto("columna", 5.5, 5.5, 4.0, [200, 200, 200], true);
    c.objeto("arco", 3.5, 3.5, 3.0, [200, 200, 200], false);
    c.aparicion(2.5, 2.5);
    let t = c.construir().a_terreno();
    let k = 5 * 12 + 5;
    assert!(t.solido[k], "la columna es pared para los demás visores");
    assert_eq!(t.alturas[k], 4, "y tan alta como la columna");
    assert!(!t.solido[3 * 12 + 3], "el arco no bloquea");
    assert_eq!(t.techo[2 * 12 + 2], SIN_TECHO);
    assert!(t.cielo[2 * 12 + 2]);
}

#[test]
fn bienvenida_lleva_el_mapa_y_los_clientes_antiguos_la_leen() {
    let m = mundo::incluido("nexo").unwrap();
    let b = DelServidor::Bienvenida { tu_id: 100, terreno: m.a_terreno(), version: 1, modo: "doom:deathmatch".into(), mapa: Some(m) };
    let json = serde_json::to_string(&b).unwrap();
    // Un cliente antiguo (sin el campo `mapa`) ignora lo que no conoce.
    #[derive(serde::Deserialize)]
    enum Antiguo {
        Bienvenida { tu_id: u32 },
    }
    let Antiguo::Bienvenida { tu_id } = serde_json::from_str(&json).unwrap();
    assert_eq!(tu_id, 100);
    let DelServidor::Bienvenida { mapa, .. } = serde_json::from_str(&json).unwrap() else { panic!() };
    assert_eq!(mapa.unwrap().id, "nexo");
}

/// El ejemplo de la página "Mundos nativos" de la documentación, tal cual.
#[test]
fn ejemplo_de_la_documentacion() {
    let texto = "mapa patio                     # id
nombre Patio                   # name
autor Your name                # author
licencia CC0-1.0               # license
lado 8                         # side
cielo 22 24 58  150 92 210     # sky: zenith (r g b) and horizon (r g b)
ambiente 0.3                   # ambient light

leyenda a core:pared           # legend: letter → material
leyenda b core:piedra
leyenda c core:agua

suelo                          # floor: '#' wall · '0'–'9' and 'a'–'z' = height 0–35
########
#111111#
#100001#
#100001#
#111221#
#111221#
#111111#
########
fin
techo                          # ceiling: '.' sky · digit = ceiling height
########
#......#
#......#
#......#
#444444#
#444444#
#444444#
########
fin
material                       # one legend letter per cell (walls included)
aaaaaaaa
abbbbbba
abccccba
abccccba
abbbbbba
abbbbbba
abbbbbba
aaaaaaaa
fin

luz 4 5 3  255 200 140  0.8 8          # light: x z height-above-floor  r g b  intensity range
objeto farola 1.5 1.5 3  255 210 150 solido   # object: type x z height  r g b  [solido = solid]
aparicion 1.5 6.5                      # spawn point
";
    let m = Mapa::desde_mvm(texto).unwrap();
    assert_eq!((m.lado, m.luces.len(), m.objetos.len(), m.apariciones.len()), (8, 1, 1, 1));
    let t = m.a_terreno();
    assert!(comprobar_terreno(&t).is_empty(), "{:?}", comprobar_terreno(&t));
    assert!(comprobar_reglas(&t).is_empty(), "{:?}", comprobar_reglas(&t));
    assert!(t.agua[2 * 8 + 2]);
}

#[test]
fn errores_legibles_en_mvm() {
    let e = Mapa::desde_mvm("mapa x\nlado 3\nsuelo\n###\n#0\n###\nfin\n").unwrap_err();
    assert!(e.contains("row 2"), "{e}");
}
