//! État de l'application et traitement des actions.
//!
//! Le cycle est unidirectionnel : Swift envoie une action, l'état change et
//! met éventuellement des octets en file pour le Bluetooth ; Swift les émet,
//! puis réinjecte la réponse reçue. L'interface se redessine à partir de
//! `view()`.

use crate::labels;
use crate::view::*;
use controller_core::buttons::{PhysicalInput, SwitchButton};
use controller_core::config::{Config, LedMode, StickTarget};
use controller_core::macros_engine::MacroDef;
use controller_core::protocol::{self, Request, Response};
use controller_core::stats::Stats;
use serde_json::Value;
use std::collections::VecDeque;

/// Nombre de macros accepté par le firmware (cf. `config::MAX_MACROS`).
const MAX_MACROS: usize = controller_core::config::MAX_MACROS;

#[derive(Debug, Clone, PartialEq)]
enum Conn {
    Idle,
    Scanning,
    Connecting,
    Ready,
    Unavailable(String),
}

/// Événements que Swift remonte depuis CoreBluetooth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleEvent {
    Scanning,
    Connecting,
    Ready,
    Disconnected,
    BluetoothOff,
    Unauthorized,
    Unsupported,
}

impl BleEvent {
    pub fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Scanning,
            1 => Self::Connecting,
            2 => Self::Ready,
            3 => Self::Disconnected,
            4 => Self::BluetoothOff,
            5 => Self::Unauthorized,
            6 => Self::Unsupported,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct OtaForm {
    ssid: String,
    password: String,
    url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Battery {
    millivolts: u16,
    percent: u8,
    charging: bool,
}

pub struct AppState {
    conn: Conn,
    config: Option<Config>,
    stats: Option<Stats>,
    battery: Option<Battery>,
    firmware: Option<String>,
    error: Option<String>,
    ota_progress: Option<u8>,

    tab: usize,
    shift_layer: bool,
    /// Brouillon de macro : entrées physiques sélectionnées et bouton visé.
    chord: u16,
    chord_output: SwitchButton,
    ota_form: OtaForm,

    outgoing: VecDeque<Vec<u8>>,
    /// Nombre de requêtes émises sans réponse encore reçue.
    in_flight: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            conn: Conn::Idle,
            config: None,
            stats: None,
            battery: None,
            firmware: None,
            error: None,
            ota_progress: None,
            tab: 0,
            shift_layer: false,
            chord: 0,
            chord_output: SwitchButton::X,
            ota_form: OtaForm::default(),
            outgoing: VecDeque::new(),
            in_flight: 0,
        }
    }

    // ---------------------------------------------------------- Bluetooth

    pub fn on_ble_event(&mut self, event: BleEvent) {
        match event {
            BleEvent::Scanning => self.conn = Conn::Scanning,
            BleEvent::Connecting => self.conn = Conn::Connecting,
            BleEvent::Ready => {
                self.conn = Conn::Ready;
                self.error = None;
                // Au raccordement, on récupère l'essentiel dans l'ordre.
                self.queue(Request::GetInfo);
                self.queue(Request::GetConfig);
                self.queue(Request::GetBattery);
            }
            BleEvent::Disconnected => {
                self.conn = Conn::Idle;
                self.config = None;
                self.stats = None;
                self.firmware = None;
                self.ota_progress = None;
                self.outgoing.clear();
                self.in_flight = 0;
            }
            BleEvent::BluetoothOff => {
                self.conn = Conn::Unavailable("Le Bluetooth est désactivé.".into())
            }
            BleEvent::Unauthorized => {
                self.conn = Conn::Unavailable(
                    "Autorisez le Bluetooth pour cette application dans Réglages.".into(),
                )
            }
            BleEvent::Unsupported => {
                self.conn = Conn::Unavailable("Cet appareil ne gère pas le Bluetooth LE.".into())
            }
        }
    }

    /// Signale un échec de transport (écriture refusée, délai dépassé…).
    pub fn on_ble_error(&mut self, message: &str) {
        self.error = Some(message.to_owned());
        self.in_flight = self.in_flight.saturating_sub(1);
    }

    /// Réinjecte une notification reçue de la manette.
    pub fn on_ble_data(&mut self, data: &[u8]) {
        let response = match protocol::decode_response(data) {
            Ok(r) => r,
            Err(_) => {
                self.error = Some("Réponse illisible reçue de la manette.".into());
                return;
            }
        };

        // La progression OTA arrive spontanément : elle ne répond à aucune
        // requête et ne doit donc pas en solder une.
        if let Response::OtaProgress(percent) = response {
            self.ota_progress = if percent >= 100 { None } else { Some(percent) };
            return;
        }

        self.in_flight = self.in_flight.saturating_sub(1);
        match response {
            Response::Info {
                firmware_version, ..
            } => self.firmware = Some(firmware_version.as_str().to_owned()),
            Response::Config(config) => self.config = Some(config),
            Response::Stats(stats) => self.stats = Some(stats),
            Response::Battery {
                millivolts,
                percent,
                charging,
            } => {
                self.battery = Some(Battery {
                    millivolts,
                    percent,
                    charging,
                })
            }
            Response::Ok => {}
            Response::Err(code) => {
                self.error = Some(format!("La manette a refusé la commande ({code:?})."))
            }
            Response::OtaProgress(_) => unreachable!("traité plus haut"),
        }
    }

    /// Prochaine trame à émettre en Bluetooth, s'il y en a une.
    pub fn take_outgoing(&mut self) -> Option<Vec<u8>> {
        let payload = self.outgoing.pop_front()?;
        self.in_flight += 1;
        Some(payload)
    }

    fn queue(&mut self, request: Request) {
        let mut buf = [0u8; protocol::MAX_MSG_LEN];
        match protocol::encode(&request, &mut buf) {
            Ok(n) => self.outgoing.push_back(buf[..n].to_vec()),
            Err(_) => self.error = Some("Configuration trop volumineuse pour être envoyée.".into()),
        }
    }

    /// Applique la configuration modifiée, à chaud.
    fn push_config(&mut self) {
        if let Some(config) = self.config.clone() {
            self.queue(Request::SetConfig(config));
        }
    }
}

// ------------------------------------------------------------- actions

impl AppState {
    /// Traite une action venue de l'interface : `id` désigne la commande,
    /// `value` porte la nouvelle valeur quand la ligne en a une.
    pub fn dispatch(&mut self, id: &str, value: &Value) {
        // Les identifiants composés se lisent en segments : « map.3.shift ».
        let parts: Vec<&str> = id.split('.').collect();
        match parts.as_slice() {
            ["error", "dismiss"] => self.error = None,
            ["connect"] => self.conn = Conn::Scanning,
            ["disconnect"] => self.on_ble_event(BleEvent::Disconnected),
            ["tab"] => {
                if let Some(index) = value.as_u64() {
                    self.tab = index as usize;
                }
            }
            ["layer"] => self.shift_layer = value.as_str() == Some("shift"),

            ["map", index, layer] => self.set_mapping(index, layer, value),
            ["stick", layer] => self.set_stick(layer, value),
            ["deadzone"] => self.set_config_number(value, |c, v| c.stick_deadzone = v as u16),

            ["turbo", "rate"] => {
                self.set_config_number(value, |c, v| c.turbo.rate_hz = (v as u8).clamp(1, 30))
            }
            ["turbo", key] => self.toggle_turbo(key),

            // Le motif littéral doit précéder le motif générique, sinon il
            // ne serait jamais atteint.
            ["chord", "output"] => {
                if let Some(button) = value.as_str().and_then(labels::switch_button_from_key) {
                    self.chord_output = button;
                }
            }
            ["chord", key] => self.toggle_chord(key),
            ["macro", "add"] => self.add_macro(),
            ["macro", "remove", index] => self.remove_macro(index),

            ["leds", "mode"] => self.set_led_mode(value),
            ["leds", "color"] => self.set_led_color(value),
            ["leds", "brightness"] => {
                self.set_config_number(value, |c, v| c.leds.brightness = v as u8)
            }
            ["haptics", "enabled"] => self.set_config_bool(value, |c, v| c.haptics.enabled = v),
            ["haptics", "strength"] => {
                self.set_config_number(value, |c, v| c.haptics.strength = (v as u8).min(127))
            }
            ["haptics", "click"] => {
                self.set_config_bool(value, |c, v| c.haptics.click_on_press = v)
            }
            ["haptics", "test"] => self.queue(Request::TestHaptic(1)),

            ["save"] => self.queue(Request::SaveConfig),
            ["identify"] => self.queue(Request::Identify),
            ["battery", "refresh"] => self.queue(Request::GetBattery),
            ["stats", "refresh"] => self.queue(Request::GetStats),
            ["stats", "reset"] => {
                self.queue(Request::ResetStats);
                self.queue(Request::GetStats);
            }
            ["factory", "reset"] => {
                self.queue(Request::FactoryReset);
                self.queue(Request::GetConfig);
            }

            ["ota", "ssid"] => self.ota_form.ssid = string_of(value),
            ["ota", "password"] => self.ota_form.password = string_of(value),
            ["ota", "url"] => self.ota_form.url = string_of(value),
            ["ota", "start"] => self.start_ota(),

            _ => self.error = Some(format!("Action inconnue : {id}")),
        }
    }

    fn set_mapping(&mut self, index: &str, layer: &str, value: &Value) {
        let Ok(index) = index.parse::<usize>() else {
            return;
        };
        let button = value.as_str().and_then(labels::switch_button_from_key);
        let shift = layer == "shift";
        let Some(config) = self.config.as_mut() else {
            return;
        };
        let target = if shift {
            &mut config.layer_shift
        } else {
            &mut config.layer_normal
        };
        let Some(slot) = target.get_mut(index) else {
            return;
        };
        *slot = button;
        self.push_config();
    }

    fn set_stick(&mut self, layer: &str, value: &Value) {
        let target = match value.as_str() {
            Some("Left") => StickTarget::Left,
            Some("Right") => StickTarget::Right,
            _ => return,
        };
        let Some(config) = self.config.as_mut() else {
            return;
        };
        if layer == "shift" {
            config.stick_shift = target;
        } else {
            config.stick_normal = target;
        }
        self.push_config();
    }

    fn toggle_turbo(&mut self, key: &str) {
        let Some(button) = labels::switch_button_from_key(key) else {
            return;
        };
        let Some(config) = self.config.as_mut() else {
            return;
        };
        config.turbo.enabled_mask ^= button.mask();
        self.push_config();
    }

    fn toggle_chord(&mut self, key: &str) {
        let Some(index) = key.parse::<u8>().ok().and_then(PhysicalInput::from_index) else {
            return;
        };
        self.chord ^= index.mask();
    }

    fn add_macro(&mut self) {
        let chord: Vec<PhysicalInput> = PhysicalInput::ALL
            .iter()
            .copied()
            .filter(|p| self.chord & p.mask() != 0)
            .collect();
        if chord.len() < 2 {
            self.error = Some("Une macro demande au moins deux boutons.".into());
            return;
        }
        let output = self.chord_output;
        let Some(config) = self.config.as_mut() else {
            return;
        };
        if config.macros.len() >= MAX_MACROS {
            self.error = Some(format!("Limite de {MAX_MACROS} macros atteinte."));
            return;
        }
        let macro_def = MacroDef::chord_to_button(&chord, output.mask(), 60);
        if config.macros.push(macro_def).is_err() {
            self.error = Some("Impossible d'ajouter la macro.".into());
            return;
        }
        self.chord = 0;
        self.push_config();
    }

    fn remove_macro(&mut self, index: &str) {
        let Ok(index) = index.parse::<usize>() else {
            return;
        };
        let Some(config) = self.config.as_mut() else {
            return;
        };
        if index >= config.macros.len() {
            return;
        }
        config.macros.remove(index);
        self.push_config();
    }

    fn set_led_mode(&mut self, value: &Value) {
        let mode = match value.as_str() {
            Some("Off") => LedMode::Off,
            Some("Solid") => LedMode::Solid,
            Some("Breathe") => LedMode::Breathe,
            Some("Rainbow") => LedMode::Rainbow,
            Some("React") => LedMode::React,
            _ => return,
        };
        let Some(config) = self.config.as_mut() else {
            return;
        };
        config.leds.mode = mode;
        self.push_config();
    }

    fn set_led_color(&mut self, value: &Value) {
        let Some(hex) = value.as_str() else { return };
        let Some(rgb) = parse_hex_color(hex) else {
            self.error = Some("Couleur invalide.".into());
            return;
        };
        let Some(config) = self.config.as_mut() else {
            return;
        };
        (config.leds.r, config.leds.g, config.leds.b) = rgb;
        self.push_config();
    }

    fn set_config_number(&mut self, value: &Value, apply: impl FnOnce(&mut Config, f64)) {
        let Some(number) = value.as_f64() else { return };
        let Some(config) = self.config.as_mut() else {
            return;
        };
        apply(config, number);
        self.push_config();
    }

    fn set_config_bool(&mut self, value: &Value, apply: impl FnOnce(&mut Config, bool)) {
        let Some(flag) = value.as_bool() else { return };
        let Some(config) = self.config.as_mut() else {
            return;
        };
        apply(config, flag);
        self.push_config();
    }

    fn start_ota(&mut self) {
        let form = self.ota_form.clone();
        let fields = [
            (form.ssid.as_str(), 32usize, "Le nom du réseau"),
            (form.password.as_str(), 64, "Le mot de passe"),
            (form.url.as_str(), 128, "L'adresse du firmware"),
        ];
        for (text, limit, what) in fields {
            if text.len() > limit {
                self.error = Some(format!("{what} dépasse {limit} caractères."));
                return;
            }
        }
        let request = Request::StartOta {
            ssid: heapless::String::try_from(form.ssid.as_str()).unwrap_or_default(),
            password: heapless::String::try_from(form.password.as_str()).unwrap_or_default(),
            url: heapless::String::try_from(form.url.as_str()).unwrap_or_default(),
        };
        self.queue(request);
        self.ota_progress = Some(0);
    }
}

fn string_of(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

/// `#rrggbb` → composantes. Tolère l'absence de dièse.
fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}

// --------------------------------------------------------- modèle de vue

impl AppState {
    pub fn view(&self) -> View {
        View {
            screen: match (&self.conn, &self.config) {
                (Conn::Ready, Some(config)) => Screen::Tabs {
                    tabs: self.tabs(config),
                },
                _ => self.connect_screen(),
            },
            banner: self.ota_progress.map(|percent| Banner::Ota {
                percent,
                title: format!("Mise à jour du firmware — {percent} %"),
                message: "Ne coupez pas la manette.".into(),
            }),
            error: self.error.clone(),
            busy: self.in_flight > 0,
        }
    }

    fn connect_screen(&self) -> Screen {
        let (message, spinner, action) = match &self.conn {
            Conn::Idle => (
                "Allumez la manette, puis lancez la recherche.".to_string(),
                false,
                Some(button("connect", "Rechercher la manette")),
            ),
            Conn::Scanning => ("Recherche de la manette…".to_string(), true, None),
            Conn::Connecting => ("Connexion en cours…".to_string(), true, None),
            // Raccordé mais configuration pas encore reçue.
            Conn::Ready => ("Lecture de la configuration…".to_string(), true, None),
            Conn::Unavailable(why) => (why.clone(), false, None),
        };
        Screen::Connect {
            title: "Nexus One".into(),
            message,
            spinner,
            action,
        }
    }

    fn tabs(&self, config: &Config) -> Vec<Tab> {
        let definitions: [(&str, &str, &str, Vec<Section>); 5] = [
            (
                "mapping",
                "Boutons",
                "square.grid.2x2",
                self.mapping_sections(config),
            ),
            ("turbo", "Turbo", "bolt", self.turbo_sections(config)),
            (
                "macros",
                "Macros",
                "wand.and.stars",
                self.macros_sections(config),
            ),
            ("stats", "Stats", "chart.bar", self.stats_sections()),
            (
                "settings",
                "Réglages",
                "gearshape",
                self.settings_sections(config),
            ),
        ];
        definitions
            .into_iter()
            .enumerate()
            .map(|(index, (id, title, icon, sections))| Tab {
                id: id.into(),
                title: title.into(),
                icon: icon.into(),
                selected: index == self.tab,
                sections,
            })
            .collect()
    }

    fn mapping_sections(&self, config: &Config) -> Vec<Section> {
        let layer = if self.shift_layer { "shift" } else { "normal" };
        let mut rows = Vec::new();
        for input in PhysicalInput::ALL {
            if input == PhysicalInput::TurboMod || input == PhysicalInput::ShiftMod {
                continue;
            }
            let current = if self.shift_layer {
                config.layer_shift[input as usize]
            } else {
                config.layer_normal[input as usize]
            };
            rows.push(Row::Picker {
                id: format!("map.{}.{}", input as usize, layer),
                label: labels::physical(input).into(),
                value: current.map(labels::switch_button_key).unwrap_or("").into(),
                options: button_options(true),
            });
        }

        let stick_target = if self.shift_layer {
            config.stick_shift
        } else {
            config.stick_normal
        };
        vec![
            Section::bare(vec![Row::Segmented {
                id: "layer".into(),
                value: layer.into(),
                options: vec![option("normal", "Normale"), option("shift", "SHIFT")],
            }])
            .with_footer(if self.shift_layer {
                "Ce que font les boutons tant que le modificateur SHIFT est maintenu."
            } else {
                "Ce que font les boutons en temps normal."
            }),
            Section::new("Boutons", rows),
            Section::new(
                "Joystick",
                vec![
                    Row::Picker {
                        id: format!("stick.{layer}"),
                        label: "Envoie vers".into(),
                        value: match stick_target {
                            StickTarget::Left => "Left".into(),
                            StickTarget::Right => "Right".into(),
                        },
                        options: vec![
                            option("Left", "Stick gauche"),
                            option("Right", "Stick droit"),
                        ],
                    },
                    Row::Slider {
                        id: "deadzone".into(),
                        label: format!("Zone morte : {} ‰", config.stick_deadzone),
                        value: config.stick_deadzone as f64,
                        min: 0.0,
                        max: 400.0,
                        step: 10.0,
                    },
                ],
            ),
            save_section(),
        ]
    }

    fn turbo_sections(&self, config: &Config) -> Vec<Section> {
        let rows = SwitchButton::DISPLAY_ORDER
            .iter()
            .map(|button| Row::Toggle {
                id: format!("turbo.{}", labels::switch_button_key(*button)),
                label: labels::switch_button(*button).into(),
                value: config.turbo.enabled_mask & button.mask() != 0,
            })
            .collect();
        vec![
            Section::bare(vec![Row::Stepper {
                id: "turbo.rate".into(),
                label: format!("Cadence : {} appuis/s", config.turbo.rate_hz),
                value: config.turbo.rate_hz as f64,
                min: 1.0,
                max: 30.0,
            }])
            .with_footer(
                "Sur la manette : maintenez TURBO et appuyez sur un bouton pour activer sa rafale, sans passer par l'application.",
            ),
            Section::new("Boutons en rafale", rows),
            save_section(),
        ]
    }

    fn macros_sections(&self, config: &Config) -> Vec<Section> {
        let mut existing: Vec<Row> = Vec::new();
        if config.macros.is_empty() {
            existing.push(Row::Text {
                label: "Aucune macro.".into(),
                value: None,
            });
        }
        for (index, macro_def) in config.macros.iter().enumerate() {
            let inputs: Vec<&str> = PhysicalInput::ALL
                .iter()
                .filter(|p| macro_def.trigger_mask & p.mask() != 0)
                .map(|p| labels::physical(*p))
                .collect();
            let emitted: Vec<&str> = macro_def
                .steps
                .first()
                .map(|step| {
                    SwitchButton::DISPLAY_ORDER
                        .iter()
                        .filter(|b| step.buttons_mask & b.mask() != 0)
                        .map(|b| labels::switch_button(*b))
                        .collect()
                })
                .unwrap_or_default();
            existing.push(Row::Text {
                label: format!("{} → {}", inputs.join(" + "), emitted.join(" + ")),
                value: None,
            });
            existing.push(Row::Button {
                id: format!("macro.remove.{index}"),
                label: "Supprimer".into(),
                destructive: true,
                disabled: false,
                confirm: None,
            });
        }

        let mut draft: Vec<Row> = PhysicalInput::ALL
            .iter()
            .filter(|p| **p != PhysicalInput::TurboMod && **p != PhysicalInput::ShiftMod)
            .map(|input| Row::Toggle {
                id: format!("chord.{}", *input as usize),
                label: labels::physical(*input).into(),
                value: self.chord & input.mask() != 0,
            })
            .collect();
        draft.push(Row::Picker {
            id: "chord.output".into(),
            label: "Bouton émis".into(),
            value: labels::switch_button_key(self.chord_output).into(),
            options: button_options(false),
        });
        let selected = self.chord.count_ones();
        draft.push(Row::Button {
            id: "macro.add".into(),
            label: "Ajouter la macro".into(),
            destructive: false,
            disabled: selected < 2 || config.macros.len() >= MAX_MACROS,
            confirm: None,
        });

        vec![
            Section::new("Macros enregistrées", existing),
            Section::new("Nouvelle macro", draft).with_footer(format!(
                "Choisissez au moins deux boutons. {} macro(s) sur {MAX_MACROS}.",
                config.macros.len()
            )),
            save_section(),
        ]
    }

    fn stats_sections(&self) -> Vec<Section> {
        let Some(stats) = &self.stats else {
            return vec![Section::bare(vec![
                Row::Text {
                    label: "Aucune statistique chargée.".into(),
                    value: None,
                },
                button("stats.refresh", "Charger les statistiques"),
            ])];
        };
        let maximum = stats.presses.iter().copied().max().unwrap_or(1).max(1);
        let gauges = PhysicalInput::ALL
            .iter()
            .map(|input| {
                let count = stats.presses[*input as usize];
                Row::Gauge {
                    label: labels::physical(*input).into(),
                    value: count as f64,
                    max: maximum as f64,
                    detail: count.to_string(),
                }
            })
            .collect();
        vec![
            Section::bare(vec![
                Row::Text {
                    label: "Temps de jeu".into(),
                    value: Some(labels::duration(stats.uptime_s)),
                },
                Row::Text {
                    label: "Macros déclenchées".into(),
                    value: Some(stats.macros_fired.to_string()),
                },
            ]),
            Section::new("Appuis par bouton", gauges),
            Section::bare(vec![
                button("stats.refresh", "Actualiser"),
                Row::Button {
                    id: "stats.reset".into(),
                    label: "Remettre les compteurs à zéro".into(),
                    destructive: true,
                    disabled: false,
                    confirm: Some(Confirm {
                        title: "Remettre les compteurs à zéro ?".into(),
                        message: "Les statistiques accumulées seront perdues.".into(),
                        action_label: "Remettre à zéro".into(),
                    }),
                },
            ]),
        ]
    }

    fn settings_sections(&self, config: &Config) -> Vec<Section> {
        let mut controller_rows = Vec::new();
        if let Some(firmware) = &self.firmware {
            controller_rows.push(Row::Text {
                label: "Firmware".into(),
                value: Some(firmware.clone()),
            });
        }
        if let Some(battery) = &self.battery {
            controller_rows.push(Row::Text {
                label: "Batterie".into(),
                value: Some(format!(
                    "{} % · {:.2} V{}",
                    battery.percent,
                    battery.millivolts as f64 / 1000.0,
                    if battery.charging { " ⚡" } else { "" }
                )),
            });
        }
        controller_rows.push(button("battery.refresh", "Actualiser la batterie"));
        controller_rows.push(button("identify", "Identifier la manette"));
        controller_rows.push(button("disconnect", "Se déconnecter"));

        vec![
            Section::new(
                "Éclairage",
                vec![
                    Row::Picker {
                        id: "leds.mode".into(),
                        label: "Mode".into(),
                        value: led_mode_key(config.leds.mode).into(),
                        options: vec![
                            option("Off", "Éteint"),
                            option("Solid", "Fixe"),
                            option("Breathe", "Respiration"),
                            option("Rainbow", "Arc-en-ciel"),
                            option("React", "Réagit aux appuis"),
                        ],
                    },
                    Row::Color {
                        id: "leds.color".into(),
                        label: "Couleur".into(),
                        value: format!(
                            "#{:02x}{:02x}{:02x}",
                            config.leds.r, config.leds.g, config.leds.b
                        ),
                    },
                    Row::Slider {
                        id: "leds.brightness".into(),
                        label: format!("Luminosité : {}", config.leds.brightness),
                        value: config.leds.brightness as f64,
                        min: 0.0,
                        max: 255.0,
                        step: 1.0,
                    },
                ],
            ),
            Section::new(
                "Vibrations",
                vec![
                    Row::Toggle {
                        id: "haptics.enabled".into(),
                        label: "Activées".into(),
                        value: config.haptics.enabled,
                    },
                    Row::Slider {
                        id: "haptics.strength".into(),
                        label: format!("Force : {}", config.haptics.strength),
                        value: config.haptics.strength as f64,
                        min: 0.0,
                        max: 127.0,
                        step: 1.0,
                    },
                    Row::Toggle {
                        id: "haptics.click".into(),
                        label: "Clic à chaque appui".into(),
                        value: config.haptics.click_on_press,
                    },
                    button("haptics.test", "Tester la vibration"),
                ],
            ),
            Section::new("Manette", controller_rows),
            Section::new(
                "Mise à jour du firmware",
                vec![
                    Row::Field {
                        id: "ota.ssid".into(),
                        label: "Réseau WiFi".into(),
                        value: self.ota_form.ssid.clone(),
                        placeholder: "Nom du réseau".into(),
                        secure: false,
                        keyboard: "default".into(),
                    },
                    Row::Field {
                        id: "ota.password".into(),
                        label: "Mot de passe".into(),
                        value: self.ota_form.password.clone(),
                        placeholder: String::new(),
                        secure: true,
                        keyboard: "default".into(),
                    },
                    Row::Field {
                        id: "ota.url".into(),
                        label: "Adresse du firmware".into(),
                        value: self.ota_form.url.clone(),
                        placeholder: "https://…/nexus-one.bin".into(),
                        secure: false,
                        keyboard: "url".into(),
                    },
                    Row::Button {
                        id: "ota.start".into(),
                        label: "Lancer la mise à jour".into(),
                        destructive: false,
                        disabled: self.ota_form.ssid.is_empty() || self.ota_form.url.is_empty(),
                        confirm: Some(Confirm {
                            title: "Mettre à jour le firmware ?".into(),
                            message: "Ne coupez pas la manette pendant l'opération.".into(),
                            action_label: "Mettre à jour".into(),
                        }),
                    },
                ],
            )
            .with_footer("La manette rejoint votre WiFi, télécharge le firmware, puis redémarre."),
            Section::bare(vec![Row::Button {
                id: "factory.reset".into(),
                label: "Réglages d'usine".into(),
                destructive: true,
                disabled: false,
                confirm: Some(Confirm {
                    title: "Restaurer les réglages d'usine ?".into(),
                    message: "Le remappage, le turbo et les macros seront perdus.".into(),
                    action_label: "Restaurer".into(),
                }),
            }]),
            save_section(),
        ]
    }
}

fn option(value: &str, label: &str) -> Option_ {
    Option_ {
        value: value.into(),
        label: label.into(),
    }
}

fn button(id: &str, label: &str) -> Row {
    Row::Button {
        id: id.into(),
        label: label.into(),
        destructive: false,
        disabled: false,
        confirm: None,
    }
}

fn save_section() -> Section {
    Section::bare(vec![button("save", "Enregistrer sur la manette")]).with_footer(
        "Les changements s'appliquent tout de suite. L'enregistrement les conserve après extinction.",
    )
}

/// Liste des boutons pour un sélecteur ; `with_none` ajoute « aucun ».
fn button_options(with_none: bool) -> Vec<Option_> {
    let mut options = Vec::new();
    if with_none {
        options.push(option("", "— aucun —"));
    }
    options.extend(
        SwitchButton::DISPLAY_ORDER
            .iter()
            .map(|b| option(labels::switch_button_key(*b), labels::switch_button(*b))),
    );
    options
}

fn led_mode_key(mode: LedMode) -> &'static str {
    match mode {
        LedMode::Off => "Off",
        LedMode::Solid => "Solid",
        LedMode::Breathe => "Breathe",
        LedMode::Rainbow => "Rainbow",
        LedMode::React => "React",
    }
}
