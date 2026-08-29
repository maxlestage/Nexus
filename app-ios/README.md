# App iPhone — Nexus One (100 % Rust)

Application de configuration de la manette : remapping des boutons,
turbo, macros, LEDs, vibrations, statistiques, batterie et mise à jour
OTA. Construite avec [Dioxus](https://dioxuslabs.com) (UI) et
[btleplug](https://github.com/deviceplug/btleplug) (CoreBluetooth).

## Prérequis — un Mac est obligatoire

Le SDK iOS n'est distribué que sur macOS et Apple n'autorise sa compilation
que sur du matériel Apple : cette étape ne peut pas être faite sur une
machine Linux, ni dans un conteneur.

- **macOS avec Xcode** installé (fournit le SDK iOS et la signature).
- Un **compte développeur Apple** — le compte gratuit suffit pour installer
  sur son propre iPhone ; l'application expire alors au bout de 7 jours et
  se réinstalle d'un simple `dx build`.
- Les outils Rust :

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo install dioxus-cli@0.7.10      # doit correspondre à la version du crate
xcode-select --install
```

## Lancer sur iPhone

```bash
cd app-ios
dx build --platform ios --release
```

Puis, la première fois, ouvrez le projet Xcode généré pour choisir votre
équipe de signature (Signing & Capabilities → Team), branchez l'iPhone et
lancez. Sur l'iPhone, la première ouverture demande d'autoriser le
développeur dans Réglages → Général → VPN et gestion de l'appareil.

```bash
dx serve --platform ios          # simulateur — mais sans Bluetooth
```

## État de vérification

Le code de l'application **compile sans erreur ni avertissement** (vérifié
avec `cargo check` sur Linux, ce qui valide toute la logique Rust, l'usage
de l'API Dioxus et le client BLE). Ce que cette vérification ne couvre pas,
faute de Mac : la compilation vers `aarch64-apple-ios`, la signature, et le
comportement réel de CoreBluetooth sur l'appareil.

Le Bluetooth ne fonctionne pas dans le simulateur iOS : tester sur un
appareil réel. Dans le projet Xcode généré, vérifier que `Info.plist`
contient :

```xml
<key>NSBluetoothAlwaysUsageDescription</key>
<string>Connexion à la manette Nexus One pour la configurer.</string>
```

## Fonctionnement

L'app parle au service BLE `6e400001-c352-…` de la manette (voir
`controller-core/src/protocol.rs`) : chaque écran encode des messages
`Request`/`Response` en postcard — exactement le même code Rust que le
firmware, donc pas de dérive de protocole possible.
