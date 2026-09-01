#!/usr/bin/env bash
# Génère les icônes PNG à partir de web/public/favicon.svg.
#
# Le SVG source est mis à l'échelle par une page HTML de la taille exacte
# voulue : dessiner le SVG directement dans une fenêtre plus petite le
# recadrerait au lieu de le réduire.
#
# Les icônes iOS doivent être OPAQUES (pas d'alpha) et SANS coins arrondis :
# le système applique lui-même son masque. On rend donc sur fond plein.
#
# Requiert un Chromium sans interface. Usage :
#   CHROME=/chemin/vers/chrome ./tools/make-icons.sh
set -euo pipefail

cd "$(dirname "$0")/.."
SRC="public/favicon.svg"
CHROME="${CHROME:-$(command -v chromium || command -v chromium-browser || command -v google-chrome || true)}"
[ -n "$CHROME" ] || { echo "Chromium introuvable : renseignez CHROME=..." >&2; exit 1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

render() {  # taille, fichier de sortie, svg source
  local size="$1" out="$2" svg="$3"
  {
    printf '<!doctype html><meta charset="utf-8"><style>'
    printf 'html,body{margin:0;padding:0;width:%spx;height:%spx;overflow:hidden;background:#000}' "$size" "$size"
    printf 'svg{display:block;width:%spx;height:%spx}</style>' "$size" "$size"
    # Le CSS ci-dessus surcharge les attributs width/height du SVG : le
    # viewBox pilote donc l'échelle. (Ne pas les retirer par sed : cela
    # viderait aussi les rectangles de fond.)
    cat "$svg"
  } > "$TMP/page.html"
  "$CHROME" --headless --disable-gpu --no-sandbox --hide-scrollbars \
    --force-device-scale-factor=1 --window-size="$size,$size" \
    --virtual-time-budget=4000 --screenshot="$out" "file://$TMP/page.html" >/dev/null 2>&1
  echo "  $out  ${size}×${size}"
}

# Variante masquable : motif réduit à 72 % pour la zone sûre d'Android.
python3 - "$SRC" > "$TMP/maskable.svg" <<'PY'
import re, sys
svg = open(sys.argv[1]).read()
# encapsule le dessin (hors fonds pleine page) dans une homothétie centrée
svg = svg.replace('<circle cx="256" cy="256" r="80"', '<g transform="translate(256 256) scale(0.72) translate(-256 -256)"><circle cx="256" cy="256" r="80"')
svg = svg.replace('</svg>', '</g></svg>')
sys.stdout.write(svg)
PY

echo "Génération des icônes :"
render 512 public/icon-512.png            "$SRC"
render 192 public/icon-192.png            "$SRC"
render 180 public/apple-touch-icon.png    "$SRC"
render 512 public/icon-maskable-512.png   "$TMP/maskable.svg"
echo "Terminé."
