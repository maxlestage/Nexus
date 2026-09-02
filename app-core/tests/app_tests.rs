//! L'intérêt d'avoir mis l'interface en Rust : elle se teste ici, sans
//! iPhone, sans Xcode et sans manette.

use controller_core::buttons::{PhysicalInput as P, SwitchButton as S};
use controller_core::config::Config;
use controller_core::protocol::{self, ErrorCode, Request, Response};
use nexus_app_core::state::BleEvent;
use nexus_app_core::view::{Banner, Row, Screen, Tab};
use nexus_app_core::AppState;
use serde_json::{json, Value};

/// Amène l'application jusqu'à « connectée, configuration reçue », comme le
/// ferait une vraie session Bluetooth.
fn connected() -> AppState {
    let mut app = AppState::new();
    app.on_ble_event(BleEvent::Ready);
    let mut buf = [0u8; protocol::MAX_MSG_LEN];

    app.take_outgoing().expect("GetInfo");
    let n = protocol::encode(
        &Response::Info {
            protocol_version: 1,
            firmware_version: heapless::String::try_from("0.1.0").unwrap(),
            name: heapless::String::try_from("Nexus One").unwrap(),
        },
        &mut buf,
    )
    .unwrap();
    app.on_ble_data(&buf[..n]);

    app.take_outgoing().expect("GetConfig");
    let n = protocol::encode(&Response::Config(Config::default()), &mut buf).unwrap();
    app.on_ble_data(&buf[..n]);

    app.take_outgoing().expect("GetBattery");
    let n = protocol::encode(
        &Response::Battery {
            millivolts: 3900,
            percent: 76,
            charging: false,
        },
        &mut buf,
    )
    .unwrap();
    app.on_ble_data(&buf[..n]);
    app
}

fn last_request(app: &mut AppState) -> Request {
    let mut last = None;
    while let Some(payload) = app.take_outgoing() {
        last = Some(payload);
    }
    protocol::decode_request(&last.expect("une requête devait partir")).unwrap()
}

fn tabs(app: &AppState) -> Vec<Tab> {
    match app.view().screen {
        Screen::Tabs { tabs } => tabs,
        other => panic!("écran inattendu : {other:?}"),
    }
}

fn row_id(row: &Row) -> Option<&str> {
    match row {
        Row::Picker { id, .. }
        | Row::Toggle { id, .. }
        | Row::Slider { id, .. }
        | Row::Stepper { id, .. }
        | Row::Button { id, .. }
        | Row::Field { id, .. }
        | Row::Color { id, .. }
        | Row::Segmented { id, .. } => Some(id),
        Row::Text { .. } | Row::Gauge { .. } => None,
    }
}

fn find_row(app: &AppState, id: &str) -> Row {
    for tab in tabs(app) {
        for section in tab.sections {
            for row in section.rows {
                if row_id(&row) == Some(id) {
                    return row;
                }
            }
        }
    }
    panic!("ligne « {id} » absente du modèle de vue");
}

#[test]
fn starts_on_the_connect_screen() {
    let app = AppState::new();
    match app.view().screen {
        Screen::Connect {
            action, spinner, ..
        } => {
            assert!(!spinner);
            assert!(action.is_some(), "un bouton de recherche doit être proposé");
        }
        other => panic!("écran inattendu : {other:?}"),
    }
}

#[test]
fn bluetooth_unavailable_explains_why_and_offers_nothing() {
    let mut app = AppState::new();
    app.on_ble_event(BleEvent::Unauthorized);
    match app.view().screen {
        Screen::Connect {
            message, action, ..
        } => {
            assert!(message.contains("Réglages"));
            assert!(
                action.is_none(),
                "rien à proposer tant que l'autorisation manque"
            );
        }
        other => panic!("écran inattendu : {other:?}"),
    }
}

#[test]
fn connecting_queues_the_three_opening_requests() {
    let mut app = AppState::new();
    app.on_ble_event(BleEvent::Ready);
    let mut kinds = Vec::new();
    while let Some(payload) = app.take_outgoing() {
        kinds.push(protocol::decode_request(&payload).unwrap());
    }
    assert_eq!(
        kinds,
        vec![Request::GetInfo, Request::GetConfig, Request::GetBattery]
    );
}

#[test]
fn shows_tabs_once_configured() {
    let app = connected();
    let tabs = tabs(&app);
    assert_eq!(tabs.len(), 5);
    assert!(tabs[0].selected);
    assert_eq!(
        tabs.iter().map(|t| t.title.clone()).collect::<Vec<_>>(),
        vec!["Boutons", "Turbo", "Macros", "Stats", "Réglages"]
    );
}

#[test]
fn remapping_a_button_sends_the_new_configuration() {
    let mut app = connected();
    app.dispatch("map.1.normal", &json!("Y"));
    match last_request(&mut app) {
        Request::SetConfig(config) => {
            assert_eq!(config.layer_normal[P::FaceRight as usize], Some(S::Y))
        }
        other => panic!("requête inattendue : {other:?}"),
    }
}

#[test]
fn a_button_can_be_unmapped() {
    let mut app = connected();
    app.dispatch("map.1.normal", &json!(""));
    match last_request(&mut app) {
        Request::SetConfig(config) => {
            assert_eq!(config.layer_normal[P::FaceRight as usize], None)
        }
        other => panic!("requête inattendue : {other:?}"),
    }
}

#[test]
fn switching_layer_shows_the_shift_mapping() {
    let mut app = connected();
    app.dispatch("layer", &json!("shift"));
    match find_row(&app, "map.0.shift") {
        Row::Picker { value, .. } => assert_eq!(value, "DpadUp"),
        other => panic!("ligne inattendue : {other:?}"),
    }
}

#[test]
fn turbo_toggles_the_right_bit() {
    let mut app = connected();
    app.dispatch("turbo.A", &Value::Null);
    match last_request(&mut app) {
        Request::SetConfig(config) => assert_eq!(config.turbo.enabled_mask, S::A.mask()),
        other => panic!("requête inattendue : {other:?}"),
    }
    match find_row(&app, "turbo.A") {
        Row::Toggle { value, .. } => assert!(value),
        other => panic!("ligne inattendue : {other:?}"),
    }
}

#[test]
fn turbo_rate_is_clamped() {
    let mut app = connected();
    app.dispatch("turbo.rate", &json!(99));
    match last_request(&mut app) {
        Request::SetConfig(config) => assert_eq!(config.turbo.rate_hz, 30),
        other => panic!("requête inattendue : {other:?}"),
    }
}

#[test]
fn a_macro_needs_at_least_two_buttons() {
    let mut app = connected();
    app.dispatch(&format!("chord.{}", P::FaceRight as usize), &Value::Null);
    app.dispatch("macro.add", &Value::Null);
    assert!(app.view().error.unwrap().contains("deux boutons"));
    assert!(app.take_outgoing().is_none(), "rien ne doit partir");
}

#[test]
fn adding_a_macro_sends_it_and_clears_the_draft() {
    let mut app = connected();
    app.dispatch(&format!("chord.{}", P::FaceRight as usize), &Value::Null);
    app.dispatch(&format!("chord.{}", P::FaceBottom as usize), &Value::Null);
    app.dispatch("chord.output", &json!("X"));
    app.dispatch("macro.add", &Value::Null);

    match last_request(&mut app) {
        Request::SetConfig(config) => {
            assert_eq!(config.macros.len(), 1);
            let m = &config.macros[0];
            assert_eq!(m.trigger_mask, P::FaceRight.mask() | P::FaceBottom.mask());
            assert_eq!(m.steps[0].buttons_mask, S::X.mask());
        }
        other => panic!("requête inattendue : {other:?}"),
    }
    match find_row(&app, &format!("chord.{}", P::FaceRight as usize)) {
        Row::Toggle { value, .. } => assert!(!value, "le brouillon doit être vidé"),
        other => panic!("ligne inattendue : {other:?}"),
    }
}

#[test]
fn macro_limit_is_enforced_and_explained() {
    let mut app = connected();
    for _ in 0..controller_core::config::MAX_MACROS {
        app.dispatch(&format!("chord.{}", P::FaceRight as usize), &Value::Null);
        app.dispatch(&format!("chord.{}", P::FaceBottom as usize), &Value::Null);
        app.dispatch("macro.add", &Value::Null);
    }
    app.dispatch(&format!("chord.{}", P::FaceTop as usize), &Value::Null);
    app.dispatch(&format!("chord.{}", P::FaceLeft as usize), &Value::Null);
    app.dispatch("macro.add", &Value::Null);
    assert!(app.view().error.unwrap().contains("Limite"));
    match find_row(&app, "macro.add") {
        Row::Button { disabled, .. } => assert!(disabled),
        other => panic!("ligne inattendue : {other:?}"),
    }
}

#[test]
fn led_colour_is_parsed_and_rejected_when_invalid() {
    let mut app = connected();
    app.dispatch("leds.color", &json!("#ff8800"));
    match last_request(&mut app) {
        Request::SetConfig(config) => {
            assert_eq!(
                (config.leds.r, config.leds.g, config.leds.b),
                (0xff, 0x88, 0x00)
            )
        }
        other => panic!("requête inattendue : {other:?}"),
    }
    app.dispatch("leds.color", &json!("rouge"));
    assert!(app.view().error.unwrap().contains("Couleur invalide"));
}

#[test]
fn ota_progress_shows_a_banner_without_consuming_a_request() {
    let mut app = connected();
    app.dispatch("ota.ssid", &json!("Maison"));
    app.dispatch("ota.url", &json!("https://exemple.fr/f.bin"));
    app.dispatch("ota.start", &Value::Null);
    assert!(matches!(last_request(&mut app), Request::StartOta { .. }));

    let mut buf = [0u8; protocol::MAX_MSG_LEN];
    let n = protocol::encode(&Response::OtaProgress(42), &mut buf).unwrap();
    app.on_ble_data(&buf[..n]);
    match app.view().banner {
        Some(Banner::Ota { percent, .. }) => assert_eq!(percent, 42),
        other => panic!("bandeau inattendu : {other:?}"),
    }
    let n = protocol::encode(&Response::OtaProgress(100), &mut buf).unwrap();
    app.on_ble_data(&buf[..n]);
    assert!(app.view().banner.is_none(), "à 100 % la manette redémarre");
}

#[test]
fn ota_button_stays_disabled_until_the_form_is_filled() {
    let mut app = connected();
    match find_row(&app, "ota.start") {
        Row::Button { disabled, .. } => assert!(disabled),
        other => panic!("ligne inattendue : {other:?}"),
    }
    app.dispatch("ota.ssid", &json!("Maison"));
    app.dispatch("ota.url", &json!("https://exemple.fr/f.bin"));
    match find_row(&app, "ota.start") {
        Row::Button { disabled, .. } => assert!(!disabled),
        other => panic!("ligne inattendue : {other:?}"),
    }
}

#[test]
fn disconnecting_returns_to_the_connect_screen() {
    let mut app = connected();
    app.dispatch("disconnect", &Value::Null);
    assert!(matches!(app.view().screen, Screen::Connect { .. }));
    assert!(app.take_outgoing().is_none());
}

#[test]
fn a_refused_command_is_reported() {
    let mut app = connected();
    app.dispatch("save", &Value::Null);
    app.take_outgoing();
    let mut buf = [0u8; protocol::MAX_MSG_LEN];
    let n = protocol::encode(&Response::Err(ErrorCode::StorageFull), &mut buf).unwrap();
    app.on_ble_data(&buf[..n]);
    assert!(app.view().error.unwrap().contains("refusé"));
}

#[test]
fn unknown_action_is_surfaced_rather_than_ignored() {
    let mut app = connected();
    app.dispatch("truc.machin", &Value::Null);
    assert!(app.view().error.unwrap().contains("Action inconnue"));
}

#[test]
fn error_can_be_dismissed() {
    let mut app = connected();
    app.dispatch("truc.machin", &Value::Null);
    app.dispatch("error.dismiss", &Value::Null);
    assert!(app.view().error.is_none());
}

/// SwiftUI identifie les lignes d'une liste par leur identifiant : deux
/// lignes homonymes dans un même onglet se marcheraient dessus. D'un onglet
/// à l'autre en revanche, la répétition est normale — « save » figure au bas
/// de chaque écran de réglage.
#[test]
fn row_ids_are_unique_within_each_tab() {
    let app = connected();
    for tab in tabs(&app) {
        let mut seen = std::collections::HashSet::new();
        for section in tab.sections {
            for row in section.rows {
                if let Some(id) = row_id(&row) {
                    assert!(
                        seen.insert(id.to_owned()),
                        "identifiant en double dans l'onglet « {} » : {id}",
                        tab.title
                    );
                }
            }
        }
    }
}

#[test]
fn the_whole_view_serialises_to_json() {
    let app = connected();
    let json = serde_json::to_string(&app.view()).unwrap();
    assert!(json.contains("\"screen\""));
    assert!(
        json.contains("Pouce · haut"),
        "les libellés viennent de Rust"
    );
}
