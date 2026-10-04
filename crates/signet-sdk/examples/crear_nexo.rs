//! Genera `mundos/nexo.mvm`, el primer mundo nativo de Signet, con el
//! constructor del SDK. Sirve también de ejemplo para escribir mundos por código.
//!
//!   cargo run --example crear_nexo
//!
//! Nexo (40 × 40 m): una plaza abierta al cielo con un cristal en el centro y
//! cuatro salas, una por estilo: templo de piedra (norte), laboratorio con un
//! canal tóxico (sur), jardín con estanque (oeste) y hangar con pasarela (este).
//! Pasillos y un anillo de madera las unen.

use signet_sdk::conformidad::{comprobar_reglas, comprobar_terreno};
use signet_sdk::mundo::ConstructorMapa;

fn main() {
    let mut c = ConstructorMapa::nuevo("nexo", "Nexo", 40);
    c.autor("Signet Protocol", "CC0-1.0").cielo([22, 24, 58], [150, 92, 210], 0.28);
    let pared = c.material("core:pared");
    let piedra = c.material("core:piedra");
    let metal = c.material("core:metal");
    let tecno = c.material("core:tecnologia");
    let madera = c.material("core:madera");
    let hierba = c.material("core:hierba");
    let agua = c.material("core:agua");
    let toxico = c.material("core:toxico");
    let _ = pared;

    // Anillo de madera y los cuatro pasillos de metal (primero: las salas los pisan).
    c.sala(10, 10, 29, 11, 1, Some(4), madera).sala(10, 28, 29, 29, 1, Some(4), madera);
    c.sala(10, 10, 11, 29, 1, Some(4), madera).sala(28, 10, 29, 29, 1, Some(4), madera);
    c.sala(18, 6, 21, 13, 1, Some(4), metal).sala(18, 26, 21, 33, 1, Some(4), metal);
    c.sala(6, 18, 13, 21, 1, Some(4), metal).sala(26, 18, 33, 21, 1, Some(4), metal);

    // Norte: templo de piedra con altar elevado y columnas.
    c.sala(12, 2, 27, 8, 1, Some(7), piedra);
    c.sala(17, 2, 22, 3, 3, Some(7), piedra);
    c.escalera(17, 5, 22, 4, 2, 3, false, Some(7), piedra);
    for (x, z) in [(13.5, 3.5), (26.5, 3.5), (13.5, 7.5), (26.5, 7.5)] {
        c.objeto("columna", x, z, 6.0, [196, 186, 160], true);
    }
    c.luz(20.0, 3.0, 4.0, [255, 176, 96], 1.0, 12.0).luz(14.0, 6.0, 4.0, [255, 140, 70], 0.6, 8.0).luz(26.0, 6.0, 4.0, [255, 140, 70], 0.6, 8.0);

    // Sur: laboratorio con canal tóxico (1 m de hondo: se entra y se sale) y cajas.
    c.sala(12, 31, 27, 37, 1, Some(5), tecno);
    c.sala(13, 34, 26, 34, 0, Some(5), toxico);
    for (x, z) in [(14.5, 36.5), (16.5, 36.5), (24.5, 32.5)] {
        c.objeto("caja", x, z, 1.0, [90, 200, 230], true);
    }
    c.luz(16.0, 33.0, 3.5, [80, 220, 255], 0.9, 10.0).luz(24.0, 35.0, 3.5, [120, 255, 140], 0.9, 10.0);

    // Oeste: jardín al aire libre con estanque y farolas.
    c.sala(2, 12, 8, 27, 1, None, hierba);
    c.sala(3, 16, 5, 19, 0, None, agua);
    for (x, z) in [(7.5, 14.5), (7.5, 25.5)] {
        c.objeto("farola", x, z, 4.0, [255, 214, 140], true);
        c.luz(x, z, 3.8, [255, 214, 140], 0.8, 9.0);
    }

    // Este: hangar de metal con pasarela a 3 m.
    c.sala(31, 12, 37, 27, 1, Some(7), metal);
    c.sala(35, 13, 37, 26, 3, Some(7), metal);
    c.escalera(33, 13, 34, 26, 2, 3, true, Some(7), metal);
    c.luz(34.0, 16.0, 5.5, [200, 220, 255], 0.9, 12.0).luz(34.0, 24.0, 5.5, [200, 220, 255], 0.9, 12.0);

    // Centro: plaza abierta con estrado y el cristal del Nexo.
    c.sala(14, 14, 25, 25, 1, None, piedra);
    c.sala(18, 18, 21, 21, 2, None, tecno);
    c.objeto("cristal", 19.5, 19.5, 3.0, [176, 120, 255], true);
    c.luz(20.0, 20.0, 4.0, [170, 110, 255], 1.6, 16.0);
    for (x, z) in [(15.5, 15.5), (24.5, 15.5), (15.5, 24.5), (24.5, 24.5)] {
        c.objeto("farola", x, z, 4.0, [210, 190, 255], true);
        c.luz(x, z, 3.8, [210, 190, 255], 0.5, 7.0);
    }
    // Arcos de paso en las bocas de los pasillos (solo se ven en el visor nativo).
    for (x, z) in [(20.0, 13.5), (20.0, 26.5), (13.5, 20.0), (26.5, 20.0)] {
        c.objeto("arco", x, z, 3.0, [176, 120, 255], false);
    }
    // Luces del anillo.
    for (x, z) in [(10.5, 10.5), (29.5, 10.5), (10.5, 29.5), (29.5, 29.5)] {
        c.luz(x, z, 3.0, [255, 200, 150], 0.5, 7.0);
    }

    for (x, z) in [(14.5, 6.5), (25.5, 6.5), (14.5, 32.5), (25.5, 32.5), (4.5, 13.5), (4.5, 25.5), (32.5, 14.5), (32.5, 25.5)] {
        c.aparicion(x, z);
    }

    let mapa = c.construir();
    let t = mapa.a_terreno();
    let fallos: Vec<_> = comprobar_terreno(&t).into_iter().chain(comprobar_reglas(&t)).collect();
    for f in &fallos {
        eprintln!("FAIL {}: {}", f.codigo, f.mensaje);
    }
    assert!(fallos.is_empty(), "Nexo does not pass conformance");
    let ruta = concat!(env!("CARGO_MANIFEST_DIR"), "/mundos/nexo.mvm");
    std::fs::write(ruta, mapa.a_mvm()).expect("write nexo.mvm");
    let abiertas = (0..t.alturas.len()).filter(|k| !t.solido[*k]).count();
    println!(
        "{}: {}x{} m, {abiertas} walkable cells, {} lights, {} objects, {} spawn points → {ruta}",
        t.fuente,
        t.lado,
        t.lado,
        mapa.luces.len(),
        mapa.objetos.len(),
        mapa.apariciones.len()
    );
}
