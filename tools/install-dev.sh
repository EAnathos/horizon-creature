#!/bin/sh
# Construit le mod et l'installe dans une instance du launcher (par défaut vulture-dev), comme le ferait le
# bouton « Build » de la page Mods : la partie contenu (.hmod) et la partie native (.hnmod), puis la build
# native de l'instance. Ensuite, Jouer sur cette instance lance le jeu avec le vautour.
#
#   tools/install-dev.sh [instance]
set -eu
INSTANCE=${1:-vulture-dev}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
LAUNCHER=${HORIZON_LAUNCHER:-$HOME/Téléchargements/horizon-launcher.AppImage}
HOME_DIR=$HOME/.local/share/horizon/instances/$INSTANCE

(cd "$ROOT/content" && cargo horizon-mod build)
python3 "$ROOT/tools/pack_native.py"
cargo horizon-mod check "$ROOT/target/vulture.hnmod"
mkdir -p "$HOME_DIR/horizon-mods" "$HOME_DIR/horizon-native-mods"
cp "$ROOT/content/target/horizon-mod/vulture.hmod" "$HOME_DIR/horizon-mods/"
cp "$ROOT/target/vulture.hnmod" "$HOME_DIR/horizon-native-mods/"
grep -qx 'mod.vulture:true' "$HOME_DIR/horizon-mods.txt" 2>/dev/null || echo 'mod.vulture:true' >> "$HOME_DIR/horizon-mods.txt"
"$LAUNCHER" native-build --instance "$INSTANCE" --trust-native-code --profile dev
