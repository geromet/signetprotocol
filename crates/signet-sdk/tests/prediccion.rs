//! La predicción del cliente debe coincidir con el servidor aunque haya
//! latencia, fotogramas irregulares, giros y escaleras: se simula un servidor
//! con cola de comandos (como el de referencia) y se cuentan las correcciones.

use std::collections::VecDeque;

use signet_sdk::prediccion::Prediccion;
use signet_sdk::protocolo::{Intencion, JugadorNet, SIN_TECHO, Terreno};
use signet_sdk::reglas::{TIC, doom as reglas};

/// Pasillo de 40 m con una escalera de peldaños de 1 m en el medio y paredes alrededor.
fn escalera() -> Terreno {
    let lado = 48;
    let n = lado * lado;
    let mut alturas = vec![0i16; n];
    let mut solido = vec![true; n];
    for j in 20..28 {
        for i in 4..44 {
            let k = j * lado + i;
            solido[k] = false;
            alturas[k] = ((i as i16 - 18).clamp(0, 6)) as i16;
        }
    }
    Terreno {
        origen: [0, 0, 0],
        lado,
        alturas,
        agua: vec![false; n],
        nivel_mar: -10,
        fuente: "prueba:escalera".into(),
        techo: vec![SIN_TECHO; n],
        solido,
        material: vec![0; n],
        paleta: vec!["core:piedra".into()],
        luz: vec![200; n],
        apariciones: vec![[-18.5, 0.5]],
        cielo: vec![true; n],
    }
}

fn jugador(id: u32, pos: [f32; 3], seq: u32) -> JugadorNet {
    JugadorNet { id, pos, juego: "prueba".into(), yaw: 0.0, vida: 100, armadura: 0, arma: 1, municion: 50, frags: 0, vivo: true, seq }
}

#[test]
fn prediccion_coincide_con_el_servidor_subiendo_escaleras() {
    let t = escalera();
    let mut pred = Prediccion::default();
    // Servidor: cuerpo, cola de comandos y último aplicado.
    let (mut pos, mut y, mut seq_srv) = ((-18.5f32, 0.5f32), 0.0f32, 0u32);
    let mut cola: VecDeque<Intencion> = VecDeque::new();
    // Red: (instante de llegada, contenido) en cada sentido, 40 ms + variación.
    let mut hacia_srv: VecDeque<(f32, Intencion)> = VecDeque::new();
    let mut hacia_cli: VecDeque<(f32, JugadorNet)> = VecDeque::new();
    let mut visto = jugador(7, [pos.0, y, pos.1], 0);
    let (mut reloj, mut siguiente_tic, mut fotograma) = (0.0f32, 0.013f32, 0u32);
    let mut intencion = Intencion { yaw: -std::f32::consts::FRAC_PI_2, ..Default::default() };
    while reloj < 8.0 {
        // Fotogramas irregulares entre 4 y 20 ms.
        fotograma += 1;
        let dt = 0.004 + (fotograma * 7919 % 17) as f32 * 0.001;
        reloj += dt;
        while let Some((_, e)) = hacia_cli.front().filter(|(t, _)| *t <= reloj) {
            visto = e.clone();
            hacia_cli.pop_front();
        }
        // Entrada: andar hacia +x (la escalera), a ratos corriendo y girando un poco.
        intencion.avance = 1.0;
        intencion.correr = (reloj as u32) % 2 == 0;
        intencion.yaw = -std::f32::consts::FRAC_PI_2 + (reloj * 3.0).sin() * 0.3;
        let mut salida = Vec::new();
        pred.avanzar(&t, &visto, &intencion, dt, &mut salida);
        for c in salida {
            let latencia = 0.04 + (c.seq % 5) as f32 * 0.006;
            hacia_srv.push_back((reloj + latencia, c));
        }
        while let Some((_, c)) = hacia_srv.front().filter(|(t, _)| *t <= reloj) {
            cola.push_back(*c);
            hacia_srv.pop_front();
        }
        // Tics del servidor: hasta 3 comandos por tic, como el de referencia.
        while siguiente_tic <= reloj {
            siguiente_tic += TIC;
            for _ in 0..3 {
                let Some(c) = cola.pop_front() else { break };
                (pos, y) = reglas::mover(&t, pos, y, &c, TIC);
                seq_srv = c.seq;
            }
            hacia_cli.push_back((reloj + 0.04, jugador(7, [pos.0, y, pos.1], seq_srv)));
        }
    }
    let d = &pred.desincronia;
    assert!(d.confirmado > 100, "el servidor confirmó pocos comandos: {}", d.confirmado);
    assert_eq!(d.correcciones, 0, "hubo correcciones: {:?}", d.historial);
    assert!(y >= 5.0, "el servidor no subió la escalera (y = {y})");
}

#[test]
fn servidor_antiguo_sin_confirmaciones_no_acumula_comandos() {
    let t = escalera();
    let mut pred = Prediccion::default();
    let yo = jugador(7, [-18.5, 0.0, 0.5], 0);
    let i = Intencion { avance: 1.0, yaw: -std::f32::consts::FRAC_PI_2, ..Default::default() };
    let mut salida = Vec::new();
    let mut ultima = None;
    for _ in 0..600 {
        ultima = pred.avanzar(&t, &yo, &i, 1.0 / 60.0, &mut salida).posicion;
    }
    assert!(pred.desincronia.legado);
    // Solo se adelanta unos tics a la posición del servidor, no 10 segundos de comandos.
    let p = ultima.unwrap();
    assert!((p[0] - yo.pos[0]).abs() < 2.0, "se alejó {} m", p[0] - yo.pos[0]);
}
