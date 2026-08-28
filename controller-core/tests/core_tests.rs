use controller_core::buttons::{PhysicalInput as P, SwitchButton as S};
use controller_core::config::Config;
use controller_core::engine::{Engine, InputFrame};
use controller_core::macros_engine::MacroDef;
use controller_core::procon::{ProconIdentity, ProconProtocol};
use controller_core::protocol::{self, Request, Response};
use controller_core::report::{pack_standard_report, SwitchState, STICK_CENTER};
use controller_core::stats::Stats;

fn frame(physical: u16, now_ms: u32) -> InputFrame {
    InputFrame {
        physical,
        stick_x: 0,
        stick_y: 0,
        now_ms,
    }
}

fn engine() -> Engine {
    Engine::new(Config::default(), Stats::default())
}

#[test]
fn default_mapping_face_right_is_a() {
    let mut e = engine();
    let out = e.tick(frame(P::FaceRight.mask(), 10));
    assert_eq!(out.state.buttons, S::A.mask());
    assert!(out.press_edge);
}

#[test]
fn shift_layer_turns_face_buttons_into_dpad() {
    let mut e = engine();
    let out = e.tick(frame(P::ShiftMod.mask() | P::FaceTop.mask(), 10));
    assert_eq!(out.state.buttons, S::DpadUp.mask());
}

#[test]
fn shift_routes_stick_to_right() {
    let mut e = engine();
    let mut f = frame(P::ShiftMod.mask(), 10);
    f.stick_x = 1000;
    let out = e.tick(f);
    assert_eq!(out.state.lx, STICK_CENTER, "stick gauche au centre");
    assert!(
        out.state.rx > 0xF00,
        "stick droit à fond: {:#x}",
        out.state.rx
    );
}

#[test]
fn deadzone_zeroes_small_input() {
    let mut e = engine();
    let mut f = frame(0, 10);
    f.stick_x = 50; // sous la zone morte par défaut (80/1000)
    let out = e.tick(f);
    assert_eq!(out.state.lx, STICK_CENTER);
}

#[test]
fn turbo_toggle_then_pulse() {
    let mut e = engine();
    // TurboMod + FaceRight : bascule le turbo sur A, sans envoyer A.
    let out = e.tick(frame(P::TurboMod.mask() | P::FaceRight.mask(), 0));
    assert_eq!(out.turbo_toggled, Some(true));
    assert_eq!(out.state.buttons, 0);

    // Relâche tout puis maintient FaceRight : A doit pulser à 12 Hz
    // (période 83 ms : ON sur [0,41), OFF sur [41,83)).
    e.tick(frame(0, 10));
    let mut saw_on = false;
    let mut saw_off = false;
    for t in 100u32..300 {
        let out = e.tick(frame(P::FaceRight.mask(), t));
        if out.state.buttons & S::A.mask() != 0 {
            saw_on = true;
        } else {
            saw_off = true;
        }
    }
    assert!(saw_on && saw_off, "le turbo doit alterner ON/OFF");

    // Re-bascule pour désactiver.
    e.tick(frame(0, 400));
    let out = e.tick(frame(P::TurboMod.mask() | P::FaceRight.mask(), 410));
    assert_eq!(out.turbo_toggled, Some(false));
}

#[test]
fn macro_a_plus_b_outputs_x() {
    let mut cfg = Config::default();
    cfg.macros
        .push(MacroDef::chord_to_button(
            &[P::FaceRight, P::FaceBottom],
            S::X.mask(),
            50,
        ))
        .unwrap();
    let mut e = Engine::new(cfg, Stats::default());

    // Accord complet → X pressé, A et B masqués.
    let chord = P::FaceRight.mask() | P::FaceBottom.mask();
    let out = e.tick(frame(chord, 0));
    assert_eq!(out.state.buttons, S::X.mask());
    let out = e.tick(frame(chord, 20));
    assert_eq!(out.state.buttons, S::X.mask());
    // Après 50 ms la macro est finie ; l'accord encore tenu reste masqué.
    let out = e.tick(frame(chord, 60));
    assert_eq!(out.state.buttons, 0);
    assert_eq!(e.stats().macros_fired, 1);

    // Un appui simple sur FaceRight redonne A normalement.
    e.tick(frame(0, 100));
    let out = e.tick(frame(P::FaceRight.mask(), 110));
    assert_eq!(out.state.buttons, S::A.mask());
}

#[test]
fn stats_count_presses() {
    let mut e = engine();
    for t in 0..5u32 {
        e.tick(frame(P::FaceBottom.mask(), t * 100));
        e.tick(frame(0, t * 100 + 50));
    }
    assert_eq!(e.stats().presses[P::FaceBottom as usize], 5);
}

#[test]
fn standard_report_packs_buttons_and_sticks() {
    let mut st = SwitchState::centered();
    st.buttons = S::A.mask() | S::Zl.mask() | S::Plus.mask() | S::DpadLeft.mask();
    st.lx = 0xABC;
    st.ly = 0x123;
    let mut out = [0u8; 48];
    pack_standard_report(&st, 0x42, 8, &mut out);
    assert_eq!(out[0], 0x42);
    assert_eq!(out[1], 0x8E);
    assert_eq!(out[2], 0x08); // A
    assert_eq!(out[3], 0x02); // Plus
    assert_eq!(out[4], 0x88); // ZL | DpadLeft
    assert_eq!(out[5], 0xBC);
    assert_eq!(out[6], 0x3A); // low nibble de 0xA? + (0x123 & 0xF) << 4
    assert_eq!(out[7], 0x12);
}

#[test]
fn procon_replies_to_device_info() {
    let mut p = ProconProtocol::new(ProconIdentity {
        mac: [1, 2, 3, 4, 5, 6],
    });
    let st = SwitchState::centered();
    // 0x01 report, subcommand 0x02 (device info), rumble neutre.
    let mut req = [0u8; 12];
    req[0] = 0x01;
    req[10] = 0x02;
    let (reply, _fx) = p.handle_output_report(&req, &st, 0, 8);
    let r = reply.expect("une réponse est attendue");
    assert_eq!(r.report_id, 0x21);
    assert_eq!(r.data[12], 0x82);
    assert_eq!(r.data[13], 0x02);
    assert_eq!(r.data[16], 0x03); // type Pro Controller
}

#[test]
fn procon_spi_read_serves_stick_calibration() {
    let mut p = ProconProtocol::new(ProconIdentity { mac: [0; 6] });
    let st = SwitchState::centered();
    let mut req = [0u8; 16];
    req[0] = 0x01;
    req[10] = 0x10; // SPI read
    req[11..15].copy_from_slice(&0x603Du32.to_le_bytes());
    req[15] = 18;
    let (reply, _fx) = p.handle_output_report(&req, &st, 0, 8);
    let r = reply.unwrap();
    assert_eq!(r.data[12], 0x90);
    assert_eq!(&r.data[14..19], &[0x3D, 0x60, 0x00, 0x00, 18]);
    // Le centre 0x800 doit apparaître dans la calibration.
    assert_eq!(r.data[19 + 4], 0x08);
}

#[test]
fn procon_tracks_input_mode_and_vibration() {
    let mut p = ProconProtocol::new(ProconIdentity { mac: [0; 6] });
    let st = SwitchState::centered();
    let mut req = [0u8; 12];
    req[0] = 0x01;
    req[10] = 0x03;
    req[11] = 0x30;
    let mut req2 = [0u8; 12];
    req2[..11].copy_from_slice(&req[..11]);
    req2[10] = 0x48;
    req2[11] = 0x01;
    p.handle_output_report(&req, &st, 0, 8);
    p.handle_output_report(&req2, &st, 0, 8);
    assert_eq!(p.input_mode, 0x30);
    assert!(p.vibration_enabled);
}

#[test]
fn procon_player_lights_and_rumble() {
    let mut p = ProconProtocol::new(ProconIdentity { mac: [0; 6] });
    let st = SwitchState::centered();
    let mut req = [0u8; 12];
    req[0] = 0x01;
    // Rumble non neutre sur l'actionneur gauche.
    req[2..6].copy_from_slice(&[0x00, 0x20, 0x40, 0x40]);
    req[6..10].copy_from_slice(&[0x00, 0x01, 0x40, 0x40]);
    req[10] = 0x30; // player lights
    req[11] = 0x03; // joueur 2
    let (_reply, fx) = p.handle_output_report(&req, &st, 0, 8);
    assert_eq!(fx.player_number, Some(2));
    assert!(fx.rumble_amplitude.unwrap() > 0);
}

#[test]
fn config_roundtrip_postcard() {
    let cfg = Config::default();
    let mut buf = [0u8; 1024];
    let bytes = cfg.to_bytes(&mut buf).unwrap();
    let back = Config::from_bytes(bytes).unwrap();
    assert_eq!(cfg, back);
}

#[test]
fn protocol_roundtrip() {
    let mut buf = [0u8; protocol::MAX_MSG_LEN];
    let req = Request::SetConfig(Config::default());
    let n = protocol::encode(&req, &mut buf).unwrap();
    assert_eq!(protocol::decode_request(&buf[..n]).unwrap(), req);

    let resp = Response::Battery {
        millivolts: 3900,
        percent: 76,
        charging: false,
    };
    let n = protocol::encode(&resp, &mut buf).unwrap();
    assert_eq!(protocol::decode_response(&buf[..n]).unwrap(), resp);
}

// ---- Tests issus de l'audit ----

use controller_core::config::MAX_MACROS;
use controller_core::macros_engine::{MacroStep, MAX_STEPS};
use controller_core::report::{pack_pc_report, pack_short_report, PC_REPORT_LEN, SHORT_REPORT_LEN};

/// Une config remplacée pendant la lecture d'une macro ne doit ni paniquer
/// ni continuer à jouer l'ancienne macro (l'app envoie SetConfig à chaque
/// suppression de macro dans l'onglet Macros).
#[test]
fn set_config_during_macro_playback_is_safe() {
    let mut cfg = Config::default();
    for _ in 0..MAX_MACROS {
        cfg.macros
            .push(MacroDef::chord_to_button(
                &[P::FaceRight, P::FaceBottom],
                S::X.mask(),
                500,
            ))
            .unwrap();
    }
    let mut e = Engine::new(cfg, Stats::default());

    let chord = P::FaceRight.mask() | P::FaceBottom.mask();
    let out = e.tick(frame(chord, 0));
    assert_eq!(out.state.buttons, S::X.mask(), "la macro joue");

    // Plus aucune macro : l'ancienne lecture doit s'arrêter proprement.
    e.set_config(Config::default());
    let out = e.tick(frame(chord, 10));
    assert_eq!(
        out.state.buttons,
        S::A.mask() | S::B.mask(),
        "retour au mapping normal, sans panic"
    );
}

/// Le moteur de macros seul doit aussi survivre à un rétrécissement de la
/// liste en pleine lecture (garde-fou hors Engine).
#[test]
fn macro_engine_survives_shrunk_list() {
    use controller_core::macros_engine::MacroEngine;
    let mut macros: heapless::Vec<MacroDef, MAX_MACROS> = heapless::Vec::new();
    macros
        .push(MacroDef::chord_to_button(&[P::FaceRight], S::X.mask(), 100))
        .unwrap();
    let mut m = MacroEngine::new();
    let out = m.tick(&macros, P::FaceRight.mask(), 0, true);
    assert_eq!(out.buttons_mask, S::X.mask());

    macros.clear();
    let out = m.tick(&macros, P::FaceRight.mask(), 10, true);
    assert_eq!(out.buttons_mask, 0);
    assert!(!m.is_playing());
}

/// La configuration la plus volumineuse possible doit tenir dans un message
/// du protocole BLE (requête ET réponse), sinon toute la chaîne de config
/// casse silencieusement au-delà de quelques macros.
#[test]
fn worst_case_config_fits_ble_message() {
    let mut cfg = Config::default();
    for slot in cfg
        .layer_normal
        .iter_mut()
        .chain(cfg.layer_shift.iter_mut())
    {
        *slot = Some(S::DpadRight); // variante d'indice maximal
    }
    cfg.turbo.enabled_mask = u32::MAX;
    cfg.turbo.rate_hz = 30;
    cfg.stick_deadzone = 1000;
    cfg.macros.clear();
    for _ in 0..MAX_MACROS {
        let mut steps = heapless::Vec::new();
        for _ in 0..MAX_STEPS {
            steps
                .push(MacroStep {
                    buttons_mask: u32::MAX,
                    duration_ms: u16::MAX,
                })
                .unwrap();
        }
        cfg.macros
            .push(MacroDef {
                trigger_mask: u16::MAX,
                steps,
            })
            .unwrap();
    }

    let mut buf = [0u8; protocol::MAX_MSG_LEN];
    let req = protocol::encode(&Request::SetConfig(cfg.clone()), &mut buf)
        .expect("SetConfig pire cas doit tenir dans MAX_MSG_LEN");
    let resp = protocol::encode(&Response::Config(cfg), &mut buf)
        .expect("Response::Config pire cas doit tenir dans MAX_MSG_LEN");
    assert!(req <= protocol::MAX_MSG_LEN && resp <= protocol::MAX_MSG_LEN);
}

/// En mode PC, les quatre directions de la croix doivent être quatre
/// boutons distincts (elles écrasaient Croix↑/Croix↓ auparavant).
#[test]
fn pc_report_dpad_directions_are_distinct() {
    let dirs = [S::DpadUp, S::DpadDown, S::DpadLeft, S::DpadRight];
    let mut seen = std::collections::HashSet::new();
    for d in dirs {
        let mut st = SwitchState::centered();
        st.buttons = d.mask();
        let mut out = [0u8; PC_REPORT_LEN];
        pack_pc_report(&st, &mut out);
        let bits = u32::from(out[0]) | u32::from(out[1]) << 8 | u32::from(out[2]) << 16;
        assert_eq!(bits.count_ones(), 1, "{d:?} doit lever exactement un bit");
        assert!(
            seen.insert(bits),
            "{d:?} entre en collision avec une autre direction"
        );
    }
}

/// TURBO + accord de macro configure le turbo, il ne doit PAS déclencher
/// la macro en plus.
#[test]
fn macro_does_not_fire_while_turbo_mod_held() {
    let mut cfg = Config::default();
    cfg.macros
        .push(MacroDef::chord_to_button(
            &[P::FaceRight, P::FaceBottom],
            S::X.mask(),
            50,
        ))
        .unwrap();
    let mut e = Engine::new(cfg, Stats::default());

    let out = e.tick(frame(
        P::TurboMod.mask() | P::FaceRight.mask() | P::FaceBottom.mask(),
        0,
    ));
    assert_eq!(out.state.buttons, 0, "rien n'est envoyé à la console");
    assert_eq!(e.stats().macros_fired, 0, "la macro n'a pas été déclenchée");
    assert!(
        out.turbo_toggled.is_some(),
        "le turbo, lui, a bien été basculé"
    );
}

/// Un appui sur un modificateur seul ne doit pas produire de press_edge
/// (pas de clic haptique en prenant TURBO ou SHIFT).
#[test]
fn modifiers_alone_do_not_count_as_press() {
    let mut e = engine();
    let out = e.tick(frame(P::ShiftMod.mask(), 0));
    assert!(!out.press_edge);
    let out = e.tick(frame(P::ShiftMod.mask() | P::FaceTop.mask(), 10));
    assert!(out.press_edge, "un vrai bouton, lui, compte");
}

/// Rapport court 0x3F : boutons et hat de la croix.
#[test]
fn short_report_packs_buttons_and_hat() {
    let mut st = SwitchState::centered();
    st.buttons = S::A.mask() | S::Home.mask() | S::DpadUp.mask() | S::DpadRight.mask();
    let mut out = [0u8; SHORT_REPORT_LEN];
    pack_short_report(&st, &mut out);
    assert_eq!(out[0], 0x02); // A
    assert_eq!(out[1], 0x10); // Home
    assert_eq!(out[2], 1); // haut-droite
    assert_eq!(u16::from_le_bytes([out[3], out[4]]), 0x800 << 4);
    // Repos : hat = 8.
    let st = SwitchState::centered();
    pack_short_report(&st, &mut out);
    assert_eq!(out[2], 8);
}
