# App iPhone — Nexus One (Swift natif)

Application de configuration de la manette : remappage des deux couches,
turbo, macros, statistiques, éclairage, vibrations, batterie et mise à jour
du firmware. SwiftUI + CoreBluetooth.

## Ce que fait Swift, et ce qu'il ne fait pas

Swift ne garde que les deux responsabilités qu'aucun autre langage ne peut
assumer sur iOS : **parler à CoreBluetooth** et **dessiner à l'écran**. Tout
le reste vit dans [`app-core/`](../app-core) : l'état, les actions, les
libellés français, le protocole, et jusqu'à la description de l'interface.

`app-core` produit un **modèle de vue** en JSON — des sections et des lignes
typées (sélecteur, interrupteur, curseur, bouton…) — que `RowView.swift`
traduit mécaniquement en composants SwiftUI. Aucun texte affiché, aucune
règle d'activation ou de validation ne se trouve dans le code Swift.

```
        ┌──────────────── app-core (Rust) ────────────────┐
action  │  état → modèle de vue JSON → protocole postcard │  octets
   ▲    └─────────────────────────────────────────────────┘     │
   │                                                            ▼
SwiftUI (rendu)                                    CoreBluetooth (transport)
```

La contrepartie est considérable : **l'interface se teste sans iPhone**.
`cargo test -p nexus-app-core` vérifie 21 comportements — le bouton d'ajout
de macro reste inactif sous deux boutons choisis, la bascule turbo modifie
le bon bit, la progression OTA n'est pas comptée comme une réponse, un
identifiant de ligne n'apparaît jamais deux fois dans un onglet.

| | lignes |
|---|---|
| Rust (`app-core`, tests compris) | ~1 750 |
| Swift (transport + rendu) | ~780 |

## Construire

Le `.xcodeproj` n'est pas versionné, il se génère :

```bash
brew install xcodegen
cd ios
DEVELOPMENT_TEAM=VOTRE_TEAM_ID xcodegen generate
open NexusOne.xcodeproj
```

La compilation du pont Rust est automatique (script exécuté avant chaque
build). Depuis la ligne de commande :

```bash
fastlane check    # compile sans signer
fastlane beta     # compile, signe et envoie sur TestFlight
```

## Publication automatique

Le workflow `.github/workflows/ios.yml` compile et envoie sur TestFlight à
chaque push sur `master`. Il n'attend que **trois secrets** :

| Secret | Contenu |
|---|---|
| `ASC_ISSUER_ID` | Issuer ID de la clé App Store Connect |
| `ASC_KEY_ID` | Key ID de la clé |
| `ASC_KEY_P8` | contenu du fichier `.p8`, de `-----BEGIN` à `-----END` inclus |

Le **Team ID n'est pas demandé** : `scripts/team-id.rb` le déduit de la clé
elle-même, en lisant l'attribut `seedId` d'un identifiant de bundle du
compte — c'est exactement le Team ID. Un secret `APPLE_TEAM_ID` reste
prioritaire s'il existe, mais il est facultatif.

Aucun certificat n'est stocké dans le dépôt : Xcode crée et récupère
lui-même certificat et profil grâce à la clé (`-allowProvisioningUpdates`).
La clé est écrite dans un fichier temporaire puis effacée, y compris si le
job échoue.

### Identifiant de l'application

Par défaut `com.maximelestage.nexusone`. Pour en utiliser un autre, définir
une **variable** de dépôt `BUNDLE_ID` (Settings → Secrets and variables →
Actions → Variables) : elle alimente à la fois le projet Xcode et fastlane.

L'application doit exister dans App Store Connect avec cet identifiant pour
que l'envoi TestFlight aboutisse — c'est la seule étape à faire une fois à
la main.

Sans secrets — sur une pull request, ou avant leur configuration — le
workflow se contente d'une compilation non signée : elle valide le Swift et
le pont sans toucher au compte Apple.

## Ce qui est vérifié, et ce qui ne l'est pas

Le cœur Rust est testé (`cargo test -p nexus-app-core`, 21 tests), passe
clippy en `-D warnings` et compile pour `aarch64-apple-ios`. Le workflow
rejoue ces trois vérifications sur Linux avant même de réserver un runner
macOS.

Le code Swift, lui, n'a jamais été compilé : cela demande un Mac. La
première exécution du workflow est donc le vrai test — attendez-vous
éventuellement à quelques erreurs de compilation à corriger.

Le Bluetooth ne fonctionne pas dans le simulateur : tester sur un iPhone.
