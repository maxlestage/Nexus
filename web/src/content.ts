export type Feature = { icon: string; title: string; body: string };
export type Zone = { finger: string; what: string; maps: string; note?: string };
export type BomItem = {
  id: string;
  qty: number;
  name: string;
  role: string;
  min: number;
  max: number;
  critical?: string;
};
export type Step = { id: string; title: string; body: string };
export type PairMode = { id: string; label: string; steps: string[]; note?: string };

export const REPO = "https://github.com/maxlestage/Nexus";

export const FEATURES: Feature[] = [
  {
    icon: "hand",
    title: "Tout à une main",
    body: "Joystick et 4 boutons sous le pouce, 4 gâchettes sous l'index et le majeur, un bouton actionné par la paume. Aucune torsion du poignet.",
  },
  {
    icon: "layers",
    title: "Couche SHIFT",
    body: "Un modificateur maintenu transforme les 4 boutons du pouce en croix directionnelle et le joystick en stick droit. Deux manettes en une.",
  },
  {
    icon: "switch",
    title: "Switch sans fil",
    body: "La manette se présente à la console comme un Pro Controller officiel, en Bluetooth Classic. Rien à installer côté console.",
  },
  {
    icon: "desktop",
    title: "PC et macOS",
    body: "Une gâchette maintenue à l'allumage bascule en gamepad HID Bluetooth générique, reconnu sans pilote.",
  },
  {
    icon: "phone",
    title: "App iPhone",
    body: "Remapper chaque bouton, régler le turbo, créer des macros, changer les couleurs — sans rebrancher la manette.",
  },
  {
    icon: "wave",
    title: "Retour haptique",
    body: "Un DRV2605L traduit les vibrations envoyées par la console, et ajoute des clics de confirmation à chaque réglage.",
  },
  {
    icon: "bolt",
    title: "Turbo et macros",
    body: "TURBO + un bouton active une rafale réglable de 1 à 30 appuis par seconde. Une combinaison peut déclencher une séquence.",
  },
  {
    icon: "battery",
    title: "10 à 20 h d'autonomie",
    body: "Accu 18650 rechargeable en USB-C. Le niveau remonte à la console comme à l'application.",
  },
];

export const ZONES: Zone[] = [
  { finger: "Pouce", what: "Joystick 2 axes", maps: "Stick gauche", note: "Stick droit en couche SHIFT" },
  { finger: "Pouce", what: "4 boutons en arc", maps: "X · A · B · Y", note: "Croix directionnelle en SHIFT" },
  { finger: "Index", what: "2 gâchettes", maps: "R · ZR" },
  { finger: "Majeur", what: "2 gâchettes", maps: "L · ZL", note: "ZL à l'allumage = mode PC" },
  { finger: "Paume", what: "Gros bouton dans la poignée", maps: "Clic stick droit", note: "Actionné en fermant la main" },
  { finger: "Pouce étendu", what: "TURBO et SHIFT", maps: "Modificateurs", note: "Hors zone de repos : pas d'appui accidentel" },
];

export const BOM: BomItem[] = [
  { id: "esp32", qty: 1, name: "ESP32-WROOM-32 DevKitC", role: "Le cerveau", min: 6, max: 10, critical: "Le modèle d'origine, surtout pas un S2, S3 ou C3 : eux n'ont pas le Bluetooth Classic exigé par la Switch." },
  { id: "stick", qty: 1, name: "Joystick analogique 2 axes + clic", role: "Direction", min: 3, max: 5 },
  { id: "buttons", qty: 14, name: "Boutons tactiles 12 mm + chapeaux", role: "Entrées", min: 5, max: 10 },
  { id: "drv", qty: 1, name: "DRV2605L", role: "Pilote haptique", min: 8, max: 8 },
  { id: "motor", qty: 1, name: "Moteur vibreur ERM 3 V", role: "Vibrations", min: 2, max: 2 },
  { id: "leds", qty: 1, name: "Bandeau WS2812B 8 LEDs", role: "Éclairage", min: 3, max: 3 },
  { id: "tp4056", qty: 1, name: "TP4056 USB-C avec protection", role: "Charge", min: 2, max: 2, critical: "La version 6 broches avec DW01A + FS8205. La version 4 broches ne protège pas l'accu." },
  { id: "cell", qty: 1, name: "Accu 18650 + support", role: "Batterie", min: 5, max: 8 },
  { id: "mt3608", qty: 1, name: "Convertisseur MT3608", role: "3,7 V → 5 V", min: 2, max: 2, critical: "À régler à 5,0 V au multimètre avant de brancher quoi que ce soit." },
  { id: "switchb", qty: 1, name: "Interrupteur à glissière", role: "Marche/arrêt", min: 1, max: 1 },
  { id: "res", qty: 1, name: "Résistances 2 × 100 kΩ, 10 kΩ, 330 Ω", role: "Diviseur, rappel, protection", min: 1, max: 1 },
  { id: "wire", qty: 1, name: "Fil AWG26, veroboard, gaine", role: "Câblage", min: 3, max: 3 },
];

export const STEPS: Step[] = [
  { id: "s1", title: "Alimentation seule", body: "TP4056 + accu + interrupteur + MT3608. Réglez 5,0 V au multimètre, avec rien d'autre de branché." },
  { id: "s2", title: "ESP32 nu", body: "Alimentez la carte, flashez le firmware par USB, vérifiez que le moniteur série démarre." },
  { id: "s3", title: "Boutons, un groupe à la fois", body: "Soudez la masse en guirlande d'abord, puis les signaux. Le moniteur série affiche chaque appui." },
  { id: "s4", title: "Joystick", body: "Vérifiez que le repos est bien au centre : la calibration se fait au démarrage, stick relâché." },
  { id: "s5", title: "Haptique et LEDs", body: "Le DRV2605L en 3,3 V, le bandeau en 5 V avec son condensateur. Testez la vibration depuis l'app." },
  { id: "s6", title: "Jauge batterie", body: "Soudez le pont diviseur en dernier, puis comparez la valeur lue à celle du multimètre." },
  { id: "s7", title: "Coque LEGO", body: "Photographiez le câblage avant de refermer : cela sert de plan de remontage." },
];

export const PAIRING: PairMode[] = [
  {
    id: "switch",
    label: "Switch",
    steps: [
      "Allumez la manette : la première LED clignote en bleu.",
      "Sur la console : Manettes → Changer le style/l'ordre.",
      "« Pro Controller » apparaît dans la liste, sélectionnez-le.",
      "LED fixe et numéro de joueur affiché : c'est prêt.",
    ],
    note: "C'est le seul écran où la console accepte une nouvelle manette. Les fois suivantes, un appui sur n'importe quel bouton suffit.",
  },
  {
    id: "pc",
    label: "PC / Mac",
    steps: [
      "Allumez la manette en maintenant la gâchette majeur-bas (ZL).",
      "Windows : Paramètres → Bluetooth → Ajouter un appareil.",
      "macOS : Réglages → Bluetooth.",
      "« Nexus One » apparaît comme gamepad 16 boutons et 4 axes.",
    ],
    note: "Aucun pilote à installer : c'est du HID générique.",
  },
  {
    id: "app",
    label: "iPhone",
    steps: [
      "Ouvrez l'app Nexus One.",
      "Touchez « Se connecter à la manette ».",
      "Remappez, réglez le turbo, créez des macros.",
      "« Enregistrer » conserve les réglages après extinction.",
    ],
    note: "L'app fonctionne même pendant une partie : le BLE et le Bluetooth Classic coexistent.",
  },
];

export const ARCHITECTURE = [
  { name: "controller-core", lang: "Rust", body: "Mapping, turbo, macros, statistiques, protocole Pro Controller et protocole de configuration. Sans dépendance au matériel, donc testé sur PC.", tests: "14 tests" },
  { name: "firmware", lang: "Rust · ESP-IDF", body: "Bluetooth, HID, haptique, LEDs, mémoire NVS, mise à jour par WiFi. C'est la seule partie qui demande la carte pour être validée.", tests: "à valider sur carte" },
  { name: "app-ios", lang: "Rust · Dioxus", body: "L'application iPhone, qui parle à la manette en BLE avec exactement le même code de protocole que le firmware.", tests: "—" },
  { name: "web", lang: "TypeScript · Bun", body: "Ce site : React 19 compilé et servi par Bun, installable comme application et consultable hors ligne.", tests: "typecheck" },
];
