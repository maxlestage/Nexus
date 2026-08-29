//! Application iPhone « Nexus One » — configuration de la manette :
//! remapping des boutons, turbo, macros, LEDs, haptique, statistiques,
//! batterie et mise à jour OTA. 100 % Rust (Dioxus + CoreBluetooth).

mod ble;

use ble::BleClient;
use controller_core::buttons::{PhysicalInput, SwitchButton, NUM_PHYSICAL};
use controller_core::config::{Config, LedMode};
use controller_core::macros_engine::MacroDef;
use controller_core::protocol::{Request, Response};
use controller_core::stats::Stats;
use dioxus::prelude::*;
use futures::StreamExt;

fn main() {
    dioxus::launch(App);
}

/// Commandes envoyées par l'UI à la tâche BLE.
enum Cmd {
    Connect,
    Disconnect,
    /// Envoie la config courante à la manette (sans persister).
    Apply(Config),
    /// Persiste la config en flash.
    Save,
    FactoryReset,
    RefreshStats,
    ResetStats,
    RefreshBattery,
    TestHaptic(u8),
    Identify,
    StartOta { ssid: String, password: String, url: String },
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Mapping,
    Turbo,
    Macros,
    Stats,
    Settings,
}

#[derive(Clone, PartialEq, Default)]
struct Connection {
    connected: bool,
    busy: bool,
    firmware: String,
    error: Option<String>,
    battery_percent: u8,
    battery_mv: u16,
    charging: bool,
    ota_progress: Option<u8>,
    saved_toast: bool,
}

/// Libellés affichés à l'utilisateur.
fn physical_label(p: PhysicalInput) -> &'static str {
    match p {
        PhysicalInput::FaceTop => "Pouce · haut",
        PhysicalInput::FaceRight => "Pouce · droite",
        PhysicalInput::FaceBottom => "Pouce · bas",
        PhysicalInput::FaceLeft => "Pouce · gauche",
        PhysicalInput::IndexUpper => "Index · gâchette haute",
        PhysicalInput::IndexLower => "Index · gâchette basse",
        PhysicalInput::MiddleUpper => "Majeur · gâchette haute",
        PhysicalInput::MiddleLower => "Majeur · gâchette basse",
        PhysicalInput::Palm => "Paume",
        PhysicalInput::StickClick => "Clic du stick",
        PhysicalInput::Plus => "Bouton +",
        PhysicalInput::Minus => "Bouton −",
        PhysicalInput::Home => "Home",
        PhysicalInput::Capture => "Capture",
        PhysicalInput::TurboMod => "Modificateur TURBO",
        PhysicalInput::ShiftMod => "Modificateur SHIFT",
    }
}

fn switch_label(b: SwitchButton) -> &'static str {
    match b {
        SwitchButton::A => "A",
        SwitchButton::B => "B",
        SwitchButton::X => "X",
        SwitchButton::Y => "Y",
        SwitchButton::L => "L",
        SwitchButton::R => "R",
        SwitchButton::Zl => "ZL",
        SwitchButton::Zr => "ZR",
        SwitchButton::Plus => "+",
        SwitchButton::Minus => "−",
        SwitchButton::LStick => "Clic stick G",
        SwitchButton::RStick => "Clic stick D",
        SwitchButton::Home => "Home",
        SwitchButton::Capture => "Capture",
        SwitchButton::DpadUp => "Croix ↑",
        SwitchButton::DpadDown => "Croix ↓",
        SwitchButton::DpadLeft => "Croix ←",
        SwitchButton::DpadRight => "Croix →",
    }
}

const ALL_SWITCH: [SwitchButton; 18] = [
    SwitchButton::A,
    SwitchButton::B,
    SwitchButton::X,
    SwitchButton::Y,
    SwitchButton::L,
    SwitchButton::R,
    SwitchButton::Zl,
    SwitchButton::Zr,
    SwitchButton::Plus,
    SwitchButton::Minus,
    SwitchButton::LStick,
    SwitchButton::RStick,
    SwitchButton::Home,
    SwitchButton::Capture,
    SwitchButton::DpadUp,
    SwitchButton::DpadDown,
    SwitchButton::DpadLeft,
    SwitchButton::DpadRight,
];

const STYLE: &str = r#"
body { font-family: -apple-system, sans-serif; margin: 0; background: #101418; color: #eee; }
.top { padding: 14px; background: #182028; position: sticky; top: 0; }
.tabs { display: flex; gap: 4px; padding: 8px; background: #182028; overflow-x: auto; }
.tabs button { flex: 1; padding: 12px 8px; border: 0; border-radius: 10px; background: #223; color: #ccc; font-size: 15px; }
.tabs button.active { background: #0a84ff; color: #fff; }
.card { background: #1b232c; border-radius: 14px; margin: 10px; padding: 14px; }
.row { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 10px 0; border-bottom: 1px solid #263038; }
select, input[type=text], input[type=password], input[type=number] { background: #101820; color: #fff; border: 1px solid #345; border-radius: 8px; padding: 10px; font-size: 16px; }
button.primary { background: #0a84ff; color: #fff; border: 0; border-radius: 12px; padding: 14px; font-size: 17px; width: 100%; margin-top: 8px; }
button.danger { background: #c0392b; }
.bar { height: 14px; background: #0a84ff; border-radius: 7px; }
.muted { color: #8aa; font-size: 13px; }
"#;

#[component]
fn App() -> Element {
    let mut conn = use_signal(Connection::default);
    let mut config = use_signal(|| Option::<Config>::None);
    let mut stats = use_signal(|| Option::<Stats>::None);
    let tab = use_signal(|| Tab::Mapping);

    // Tâche BLE : possède le client, exécute les commandes de l'UI et,
    // entre deux commandes, reste à l'écoute des notifications spontanées
    // (progression OTA) pour rafraîchir l'écran sans action de l'utilisateur.
    let ble = use_coroutine(move |mut rx: UnboundedReceiver<Cmd>| async move {
        let mut client: Option<BleClient> = None;
        loop {
            let cmd = if let Some(c) = client.as_mut() {
                tokio::select! {
                    cmd = rx.next() => cmd,
                    ev = c.events.recv() => {
                        if let Some(Response::OtaProgress(p)) = ev {
                            conn.with_mut(|s| s.ota_progress = Some(p));
                        }
                        continue;
                    }
                }
            } else {
                rx.next().await
            };
            let Some(cmd) = cmd else { break };

            conn.with_mut(|c| {
                c.busy = true;
                c.error = None;
            });
            let result = handle_cmd(cmd, &mut client, &mut conn, &mut config, &mut stats).await;

            // Une erreur peut signifier que la manette est éteinte ou a
            // redémarré (fin d'OTA) : vérifier la liaison plutôt que de
            // rester affiché « connectée ».
            let mut lost = false;
            if result.is_err() {
                if let Some(c) = client.as_ref() {
                    lost = !c.is_connected().await;
                }
            }
            if lost {
                client = None;
                conn.with_mut(|s| {
                    s.connected = false;
                    s.ota_progress = None;
                });
            }
            conn.with_mut(|c| {
                c.busy = false;
                if let Err(e) = result {
                    c.error = Some(if lost {
                        "Connexion perdue — la manette est-elle allumée ?".to_string()
                    } else {
                        e.to_string()
                    });
                }
            });
        }
    });

    let c = conn.read().clone();
    rsx! {
        style { {STYLE} }
        div { class: "top",
            h2 { style: "margin:0", "Nexus One" }
            if c.connected {
                p { class: "muted",
                    "Connectée · firmware {c.firmware} · batterie {c.battery_percent}% "
                    if c.charging { "⚡" }
                }
            } else {
                p { class: "muted", "Manette non connectée" }
            }
            if c.connected {
                button {
                    class: "primary",
                    style: "background:#2c3440; margin-top:6px",
                    onclick: move |_| ble.send(Cmd::Disconnect),
                    "Se déconnecter"
                }
            }
            if let Some(err) = c.error.clone() {
                p { style: "color:#ff6b6b", "{err}" }
            }
            if let Some(p) = c.ota_progress {
                div { class: "card",
                    "Mise à jour en cours… {p}%"
                    div { class: "bar", style: "width:{p}%" }
                }
            }
            if !c.connected {
                button {
                    class: "primary",
                    disabled: c.busy,
                    onclick: move |_| ble.send(Cmd::Connect),
                    if c.busy { "Recherche…" } else { "Se connecter à la manette" }
                }
            }
        }
        if c.connected && config.read().is_some() {
            Tabs { tab }
            match *tab.read() {
                Tab::Mapping => rsx! { MappingView { config, ble } },
                Tab::Turbo => rsx! { TurboView { config, ble } },
                Tab::Macros => rsx! { MacrosView { config, ble } },
                Tab::Stats => rsx! { StatsView { stats, ble } },
                Tab::Settings => rsx! { SettingsView { config, conn, ble } },
            }
        }
    }
}

#[component]
fn Tabs(tab: Signal<Tab>) -> Element {
    let entries = [
        (Tab::Mapping, "Boutons"),
        (Tab::Turbo, "Turbo"),
        (Tab::Macros, "Macros"),
        (Tab::Stats, "Stats"),
        (Tab::Settings, "Réglages"),
    ];
    rsx! {
        div { class: "tabs",
            for (t, label) in entries {
                button {
                    class: if *tab.read() == t { "active" } else { "" },
                    onclick: move |_| tab.set(t),
                    "{label}"
                }
            }
        }
    }
}

/// Écran de remapping : pour chaque entrée physique, choisir le bouton
/// Switch émis, sur la couche normale et la couche SHIFT.
#[component]
fn MappingView(config: Signal<Option<Config>>, ble: Coroutine<Cmd>) -> Element {
    let cfg = config.read().clone().unwrap();
    let mut set_mapping = move |layer_shift: bool, idx: usize, value: String| {
        let mut cfg = config.read().clone().unwrap();
        let target = if layer_shift { &mut cfg.layer_shift } else { &mut cfg.layer_normal };
        target[idx] = value.parse::<u8>().ok().and_then(SwitchButton::from_index);
        config.set(Some(cfg.clone()));
        ble.send(Cmd::Apply(cfg));
    };
    rsx! {
        for layer_shift in [false, true] {
            div { class: "card",
                h3 { if layer_shift { "Couche SHIFT (modificateur maintenu)" } else { "Couche normale" } }
                for (i, p) in PhysicalInput::ALL.iter().copied().enumerate()
                    .filter(|(_, p)| !matches!(p, PhysicalInput::TurboMod | PhysicalInput::ShiftMod))
                {
                    div { class: "row",
                        span { {physical_label(p)} }
                        select {
                            value: {
                                let layer = if layer_shift { &cfg.layer_shift } else { &cfg.layer_normal };
                                layer[i].map(|b| (b as u8).to_string()).unwrap_or("none".into())
                            },
                            onchange: move |e| set_mapping(layer_shift, i, e.value()),
                            option { value: "none", "— aucun —" }
                            for b in ALL_SWITCH {
                                option { value: "{b as u8}", {switch_label(b)} }
                            }
                        }
                    }
                }
            }
        }
        SaveBar { ble }
    }
}

#[component]
fn TurboView(config: Signal<Option<Config>>, ble: Coroutine<Cmd>) -> Element {
    let cfg = config.read().clone().unwrap();
    rsx! {
        div { class: "card",
            h3 { "Mode Turbo" }
            p { class: "muted",
                "Sur la manette : maintenir le bouton TURBO et appuyer sur un \
                 bouton pour (dés)activer sa rafale. Ou cocher ici :"
            }
            div { class: "row",
                span { "Cadence : {cfg.turbo.rate_hz} appuis/s" }
                input {
                    r#type: "number", min: "1", max: "30",
                    value: "{cfg.turbo.rate_hz}",
                    onchange: move |e| {
                        if let Ok(v) = e.value().parse::<u8>() {
                            let mut cfg = config.read().clone().unwrap();
                            cfg.turbo.rate_hz = v.clamp(1, 30);
                            config.set(Some(cfg.clone()));
                            ble.send(Cmd::Apply(cfg));
                        }
                    }
                }
            }
            for b in ALL_SWITCH {
                div { class: "row",
                    span { {switch_label(b)} }
                    input {
                        r#type: "checkbox",
                        checked: cfg.turbo.enabled_mask & b.mask() != 0,
                        onchange: move |_| {
                            let mut cfg = config.read().clone().unwrap();
                            cfg.turbo.enabled_mask ^= b.mask();
                            config.set(Some(cfg.clone()));
                            ble.send(Cmd::Apply(cfg));
                        }
                    }
                }
            }
        }
        SaveBar { ble }
    }
}

#[component]
fn MacrosView(config: Signal<Option<Config>>, ble: Coroutine<Cmd>) -> Element {
    let cfg = config.read().clone().unwrap();
    let mut chord = use_signal(Vec::<PhysicalInput>::new);
    let mut out_button = use_signal(|| SwitchButton::X);

    rsx! {
        div { class: "card",
            h3 { "Macros existantes" }
            if cfg.macros.is_empty() {
                p { class: "muted", "Aucune macro." }
            }
            for (i, m) in cfg.macros.iter().enumerate() {
                div { class: "row",
                    span {
                        {
                            let chord_txt: Vec<&str> = PhysicalInput::ALL.iter()
                                .filter(|p| m.trigger_mask & p.mask() != 0)
                                .map(|p| physical_label(*p)).collect();
                            let out: Vec<&str> = ALL_SWITCH.iter()
                                .filter(|b| m.steps.first().map(|s| s.buttons_mask & b.mask() != 0).unwrap_or(false))
                                .map(|b| switch_label(*b)).collect();
                            format!("{} → {}", chord_txt.join(" + "), out.join("+"))
                        }
                    }
                    button {
                        class: "danger",
                        onclick: move |_| {
                            let mut cfg = config.read().clone().unwrap();
                            cfg.macros.remove(i);
                            config.set(Some(cfg.clone()));
                            ble.send(Cmd::Apply(cfg));
                        },
                        "✕"
                    }
                }
            }
        }
        div { class: "card",
            h3 { "Nouvelle macro (combinaison → bouton)" }
            p { class: "muted", "1. Choisir les boutons physiques de la combinaison :" }
            for p in PhysicalInput::ALL.iter().copied()
                .filter(|p| !matches!(p, PhysicalInput::TurboMod | PhysicalInput::ShiftMod))
            {
                div { class: "row",
                    span { {physical_label(p)} }
                    input {
                        r#type: "checkbox",
                        checked: chord.read().contains(&p),
                        onchange: move |_| {
                            let mut c = chord.read().clone();
                            if let Some(pos) = c.iter().position(|x| *x == p) {
                                c.remove(pos);
                            } else {
                                c.push(p);
                            }
                            chord.set(c);
                        }
                    }
                }
            }
            p { class: "muted", "2. Bouton Switch émis :" }
            select {
                value: "{*out_button.read() as u8}",
                onchange: move |e| {
                    if let Some(b) = e.value().parse::<u8>().ok().and_then(SwitchButton::from_index) {
                        out_button.set(b);
                    }
                },
                for b in ALL_SWITCH {
                    option { value: "{b as u8}", {switch_label(b)} }
                }
            }
            button {
                class: "primary",
                disabled: chord.read().len() < 2,
                onclick: move |_| {
                    let mut cfg = config.read().clone().unwrap();
                    let m = MacroDef::chord_to_button(&chord.read(), out_button.read().mask(), 60);
                    if cfg.macros.push(m).is_ok() {
                        config.set(Some(cfg.clone()));
                        ble.send(Cmd::Apply(cfg));
                        chord.set(Vec::new());
                    }
                },
                "Ajouter la macro"
            }
        }
        SaveBar { ble }
    }
}

#[component]
fn StatsView(stats: Signal<Option<Stats>>, ble: Coroutine<Cmd>) -> Element {
    let s = stats.read().clone();
    rsx! {
        div { class: "card",
            h3 { "Statistiques d'utilisation" }
            button { class: "primary", onclick: move |_| ble.send(Cmd::RefreshStats), "Actualiser" }
            if let Some(s) = s {
                p { class: "muted",
                    { format!("Temps de jeu : {} h {:02} min · {} macros déclenchées",
                        s.uptime_s / 3600, (s.uptime_s % 3600) / 60, s.macros_fired) }
                }
                {
                    let max = s.presses.iter().copied().max().unwrap_or(1).max(1);
                    // Pourcentages en u64 : v × 100 déborderait un u32 pour
                    // de très gros compteurs.
                    let rows: Vec<(usize, u32, u64)> = (0..NUM_PHYSICAL)
                        .map(|i| (i, s.presses[i], u64::from(s.presses[i]) * 100 / u64::from(max)))
                        .collect();
                    rsx! {
                        for (i, count, pct) in rows {
                            div { class: "row",
                                span { style: "min-width: 45%", {physical_label(PhysicalInput::ALL[i])} }
                                div { style: "flex:1",
                                    div { class: "bar", style: "width:{pct}%" }
                                }
                                span { class: "muted", "{count}" }
                            }
                        }
                    }
                }
                button {
                    class: "primary danger",
                    onclick: move |_| ble.send(Cmd::ResetStats),
                    "Remettre à zéro"
                }
            }
        }
    }
}

#[component]
fn SettingsView(
    config: Signal<Option<Config>>,
    conn: Signal<Connection>,
    ble: Coroutine<Cmd>,
) -> Element {
    let cfg = config.read().clone().unwrap();
    let mut ssid = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut url = use_signal(String::new);

    let mut apply = move |f: &dyn Fn(&mut Config)| {
        let mut cfg = config.read().clone().unwrap();
        f(&mut cfg);
        config.set(Some(cfg.clone()));
        ble.send(Cmd::Apply(cfg));
    };

    rsx! {
        div { class: "card",
            h3 { "Éclairage" }
            div { class: "row",
                span { "Mode" }
                select {
                    onchange: move |e| apply(&|c| c.leds.mode = match e.value().as_str() {
                        "solid" => LedMode::Solid,
                        "breathe" => LedMode::Breathe,
                        "rainbow" => LedMode::Rainbow,
                        "react" => LedMode::React,
                        _ => LedMode::Off,
                    }),
                    option { value: "off", selected: cfg.leds.mode == LedMode::Off, "Éteint" }
                    option { value: "solid", selected: cfg.leds.mode == LedMode::Solid, "Fixe" }
                    option { value: "breathe", selected: cfg.leds.mode == LedMode::Breathe, "Respiration" }
                    option { value: "rainbow", selected: cfg.leds.mode == LedMode::Rainbow, "Arc-en-ciel" }
                    option { value: "react", selected: cfg.leds.mode == LedMode::React, "Réagit aux appuis" }
                }
            }
            div { class: "row",
                span { "Couleur" }
                input {
                    r#type: "color",
                    value: format!("#{:02x}{:02x}{:02x}", cfg.leds.r, cfg.leds.g, cfg.leds.b),
                    onchange: move |e| apply(&|c| {
                        let v = e.value();
                        // Uniquement le format #rrggbb : tout autre valeur
                        // (chaîne vide, nom de couleur) est ignorée.
                        if v.len() == 7 && v.starts_with('#') {
                            if let (Ok(r), Ok(g), Ok(b)) = (
                                u8::from_str_radix(&v[1..3], 16),
                                u8::from_str_radix(&v[3..5], 16),
                                u8::from_str_radix(&v[5..7], 16),
                            ) {
                                c.leds.r = r;
                                c.leds.g = g;
                                c.leds.b = b;
                            }
                        }
                    })
                }
            }
            div { class: "row",
                span { "Luminosité" }
                input {
                    r#type: "range", min: "0", max: "255", value: "{cfg.leds.brightness}",
                    onchange: move |e| apply(&|c| c.leds.brightness = e.value().parse().unwrap_or(80))
                }
            }
        }
        div { class: "card",
            h3 { "Vibrations" }
            div { class: "row",
                span { "Activées" }
                input {
                    r#type: "checkbox", checked: cfg.haptics.enabled,
                    onchange: move |_| apply(&|c| c.haptics.enabled = !c.haptics.enabled)
                }
            }
            div { class: "row",
                span { "Force" }
                input {
                    r#type: "range", min: "0", max: "127", value: "{cfg.haptics.strength}",
                    onchange: move |e| apply(&|c| c.haptics.strength = e.value().parse().unwrap_or(90))
                }
            }
            div { class: "row",
                span { "Clic à chaque appui" }
                input {
                    r#type: "checkbox", checked: cfg.haptics.click_on_press,
                    onchange: move |_| apply(&|c| c.haptics.click_on_press = !c.haptics.click_on_press)
                }
            }
            button { class: "primary", onclick: move |_| ble.send(Cmd::TestHaptic(1)), "Tester la vibration" }
            button { class: "primary", onclick: move |_| ble.send(Cmd::Identify), "Identifier la manette" }
        }
        div { class: "card",
            h3 { "Batterie" }
            p { class: "muted",
                { let c = conn.read();
                  format!("{} % · {:.2} V{}", c.battery_percent, c.battery_mv as f32 / 1000.0,
                          if c.charging { " · en charge ⚡" } else { "" }) }
            }
            button { class: "primary", onclick: move |_| ble.send(Cmd::RefreshBattery), "Actualiser" }
        }
        div { class: "card",
            h3 { "Mise à jour du firmware (OTA)" }
            p { class: "muted", "La manette rejoint votre WiFi et télécharge le firmware." }
            div { class: "row", span { "WiFi (SSID)" }
                input { r#type: "text", oninput: move |e| ssid.set(e.value()) } }
            div { class: "row", span { "Mot de passe" }
                input { r#type: "password", oninput: move |e| password.set(e.value()) } }
            div { class: "row", span { "URL du firmware" }
                input { r#type: "text", placeholder: "https://…/nexus-one.bin",
                        oninput: move |e| url.set(e.value()) } }
            button {
                class: "primary",
                disabled: ssid.read().is_empty() || url.read().is_empty(),
                onclick: move |_| ble.send(Cmd::StartOta {
                    ssid: ssid.read().clone(),
                    password: password.read().clone(),
                    url: url.read().clone(),
                }),
                "Lancer la mise à jour"
            }
        }
        div { class: "card",
            button { class: "primary danger", onclick: move |_| ble.send(Cmd::FactoryReset),
                     "Réglages d'usine" }
        }
        SaveBar { ble }
    }
}

/// Barre « Enregistrer sur la manette » commune à tous les écrans d'édition.
#[component]
fn SaveBar(ble: Coroutine<Cmd>) -> Element {
    rsx! {
        div { class: "card",
            p { class: "muted",
                "Les changements sont appliqués immédiatement. \
                 « Enregistrer » les conserve après extinction."
            }
            button { class: "primary", onclick: move |_| ble.send(Cmd::Save), "Enregistrer sur la manette" }
        }
    }
}

/// Exécution des commandes côté BLE.
async fn handle_cmd(
    cmd: Cmd,
    client: &mut Option<BleClient>,
    conn: &mut Signal<Connection>,
    config: &mut Signal<Option<Config>>,
    stats: &mut Signal<Option<Stats>>,
) -> anyhow::Result<()> {
    use heapless::String as HString;

    if matches!(cmd, Cmd::Connect) {
        let mut c = BleClient::connect().await?;
        if let Response::Info { firmware_version, .. } = c.request(&Request::GetInfo).await? {
            conn.with_mut(|s| s.firmware = firmware_version.to_string());
        }
        if let Response::Config(cfg) = c.request(&Request::GetConfig).await? {
            config.set(Some(cfg));
        }
        if let Response::Battery { millivolts, percent, charging } =
            c.request(&Request::GetBattery).await?
        {
            conn.with_mut(|s| {
                s.battery_mv = millivolts;
                s.battery_percent = percent;
                s.charging = charging;
            });
        }
        *client = Some(c);
        conn.with_mut(|s| s.connected = true);
        return Ok(());
    }

    let Some(c) = client.as_mut() else {
        anyhow::bail!("non connecté");
    };

    match cmd {
        Cmd::Connect => unreachable!(),
        Cmd::Disconnect => {
            c.disconnect().await;
            *client = None;
            conn.with_mut(|s| s.connected = false);
        }
        Cmd::Apply(cfg) => {
            c.request(&Request::SetConfig(cfg)).await?;
        }
        Cmd::Save => {
            c.request(&Request::SaveConfig).await?;
        }
        Cmd::FactoryReset => {
            c.request(&Request::FactoryReset).await?;
            if let Response::Config(cfg) = c.request(&Request::GetConfig).await? {
                config.set(Some(cfg));
            }
        }
        Cmd::RefreshStats | Cmd::ResetStats => {
            if matches!(cmd, Cmd::ResetStats) {
                c.request(&Request::ResetStats).await?;
            }
            if let Response::Stats(s) = c.request(&Request::GetStats).await? {
                stats.set(Some(s));
            }
        }
        Cmd::RefreshBattery => {
            if let Response::Battery { millivolts, percent, charging } =
                c.request(&Request::GetBattery).await?
            {
                conn.with_mut(|s| {
                    s.battery_mv = millivolts;
                    s.battery_percent = percent;
                    s.charging = charging;
                });
            }
        }
        Cmd::TestHaptic(e) => {
            c.request(&Request::TestHaptic(e)).await?;
        }
        Cmd::Identify => {
            c.request(&Request::Identify).await?;
        }
        Cmd::StartOta { ssid, password, url } => {
            let req = Request::StartOta {
                ssid: HString::try_from(ssid.as_str()).map_err(|_| anyhow::anyhow!("SSID trop long"))?,
                password: HString::try_from(password.as_str())
                    .map_err(|_| anyhow::anyhow!("mot de passe trop long"))?,
                url: HString::try_from(url.as_str()).map_err(|_| anyhow::anyhow!("URL trop longue"))?,
            };
            c.request(&req).await?;
            conn.with_mut(|s| s.ota_progress = Some(0));
        }
    }
    Ok(())
}
