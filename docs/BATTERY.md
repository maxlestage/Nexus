# Batterie et charge

## Architecture

- **Accu** : Li-ion 18650 (2600–3500 mAh) dans un support, ou LiPo
  103450 (2000 mAh) plus plat. Autonomie mesurable : l'ESP32 en BT
  classique + 8 LEDs à 30 % consomment ~120–180 mA → **10 à 20 h**.
- **Charge** : module **TP4056 USB-C avec protection** (6 broches :
  IN via USB-C, B+/B− vers l'accu, OUT+/OUT− vers la manette).
  Courant de charge 1 A (par défaut) → charge complète en ~3 h.
- **5 V** : convertisseur élévateur MT3608 entre OUT et l'ESP32 (broche
  VIN/5V) + le bandeau WS2812B. Régler à 5,0 V au multimètre avant le
  premier branchement.

## Sécurité

- Prendre impérativement un TP4056 **« with protection »** (puces
  DW01A + FS8205) : coupure en surdécharge (< 2,4 V), surcharge et
  court-circuit.
- Ne jamais laisser l'accu en charge sans surveillance la première fois.
- L'accu est maintenu mécaniquement (support + poutres LEGO), jamais
  collé à chaud directement (chaleur).
- Le port USB-C de charge est celui du TP4056 ; le port USB du DevKit
  ESP32 ne sert qu'au flash. Ne pas brancher les deux en même temps la
  première fois sans avoir vérifié qu'il n'y a pas de conflit de masse
  (retirer l'accu pendant le flash par précaution).

## Jauge logicielle

Le firmware lit la tension via un pont diviseur 100 kΩ/100 kΩ sur
GPIO36 (`firmware/src/battery.rs`) :

- courbe tension → % calibrée Li-ion (4,2 V = 100 %, 3,3 V = 0 %) ;
- niveau remonté **à la Switch** (icône batterie de la console) ;
- niveau détaillé (mV + %) dans l'app iPhone ;
- LED 0 clignote rouge sous 10 %.

## Charge pendant le jeu

Possible : le TP4056 alimente la sortie pendant la charge (mode
« load sharing » approximatif). Pour un vrai load-sharing, ajouter une
diode Schottky + MOSFET-P (schéma classique) — optionnel.
