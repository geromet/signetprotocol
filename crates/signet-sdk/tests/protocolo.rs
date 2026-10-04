//! Compatibilidad del formato en la red: Signet/1 solo crece con campos opcionales,
//! así que los mensajes de versiones anteriores deben seguir leyéndose.

use signet_sdk::protocolo::{DelCliente, DelServidor, Evento, Intencion};

#[test]
fn intencion_sin_seq_de_un_adaptador_antiguo() {
    let j = r#"{"Intencion":{"avance":1.0,"lateral":0.0,"yaw":0.5,"correr":false,"disparar":true,"usar":false,"arma":1}}"#;
    let DelCliente::Intencion(i) = serde_json::from_str(j).unwrap() else { panic!() };
    assert_eq!(i.seq, 0);
    assert!(i.disparar);
}

#[test]
fn estado_sin_campos_nuevos() {
    let j = r#"{"Estado":{"jugadores":[{"id":3,"pos":[1.0,2.0,3.0],"juego":"bot"}]}}"#;
    let DelServidor::Estado { jugadores, eventos } = serde_json::from_str(j).unwrap() else { panic!() };
    assert_eq!(jugadores[0].seq, 0);
    assert!(jugadores[0].vivo, "vivo vale true por defecto");
    assert!(eventos.is_empty());
}

#[test]
fn bienvenida_con_terreno_minimo() {
    let j = r#"{"Bienvenida":{"tu_id":100,"terreno":{"origen":[0,0,0],"lado":2,"alturas":[0,1,2,3],"agua":[false,false,false,false],"nivel_mar":0}}}"#;
    let DelServidor::Bienvenida { tu_id, terreno, version, modo, mapa } = serde_json::from_str(j).unwrap() else { panic!() };
    assert!(mapa.is_none());
    assert_eq!(tu_id, 100);
    assert_eq!(version, 0);
    assert!(modo.is_empty());
    assert!(terreno.transitable(0.0, 0.0) || terreno.lado == 2);
    assert!(terreno.solido.is_empty() && terreno.paleta.is_empty());
}

#[test]
fn ida_y_vuelta_de_todos_los_mensajes() {
    let msgs = vec![
        serde_json::to_string(&DelCliente::Hola { juego: "doom".into(), version: 1 }).unwrap(),
        serde_json::to_string(&DelCliente::Posicion { x: 1.5, z: -2.0 }).unwrap(),
        serde_json::to_string(&DelCliente::Intencion(Intencion { avance: 1.0, seq: 42, ..Default::default() })).unwrap(),
    ];
    for m in msgs {
        let v: DelCliente = serde_json::from_str(&m).unwrap();
        assert_eq!(serde_json::to_string(&v).unwrap(), m);
    }
    let e = Evento::Disparo { de: 1, arma: 2, desde: [0.0; 3], hasta: [1.0; 3] };
    let s = serde_json::to_string(&e).unwrap();
    assert!(s.starts_with(r#"{"Disparo":"#), "formato externo de enum: {s}");
}
