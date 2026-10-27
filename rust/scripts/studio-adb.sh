#!/bin/sh
# Sobe o adb na porta 5037 (a do Android Studio) e lista os aparelhos.
# Rode isto e depois ligue o aparelho no MultiDroid. O Run do Studio usa
# o serial que aparecer aqui. Não crie um AVD no Studio para esse fim.
# Detalhe: docs/arquitetura/ADR-001-nucleo-nativo-e-android-studio.md
set -eu

adb=""
dados=""
studio=""
case "$(uname -s)" in
  Darwin)
    dados="$HOME/Library/Application Support/MultiDroid/sdk/platform-tools/adb"
    studio="$HOME/Library/Android/sdk/platform-tools/adb"
    ;;
  Linux)
    dados="${XDG_DATA_HOME:-$HOME/.local/share}/multidroid/sdk/platform-tools/adb"
    studio="$HOME/Android/Sdk/platform-tools/adb"
    ;;
  *)
    echo "No Windows, use rust/scripts/studio-adb.ps1" >&2
    exit 1
    ;;
esac

if [ -n "${MULTIDROID_ADB:-}" ] && [ -x "${MULTIDROID_ADB}" ]; then
  adb=$MULTIDROID_ADB
elif [ -n "${ANDROID_HOME:-}" ] && [ -x "$ANDROID_HOME/platform-tools/adb" ]; then
  adb=$ANDROID_HOME/platform-tools/adb
elif [ -n "${ANDROID_SDK_ROOT:-}" ] && [ -x "$ANDROID_SDK_ROOT/platform-tools/adb" ]; then
  adb=$ANDROID_SDK_ROOT/platform-tools/adb
elif [ -x "$dados" ]; then
  adb=$dados
elif [ -x "$studio" ]; then
  adb=$studio
elif command -v adb >/dev/null 2>&1; then
  adb=$(command -v adb)
fi

if [ -z "$adb" ]; then
  echo "Não achei o adb. Baixe as platform-tools no MultiDroid (Imagens) ou instale o Android Studio." >&2
  exit 1
fi

unset ANDROID_ADB_SERVER_PORT || true
unset ADB_SERVER_SOCKET || true
"$adb" start-server
echo "adb: $adb"
echo
"$adb" devices -l
cat <<'EOF'

No Android Studio, o aparelho do MultiDroid entra na lista do Run com o
serial emulator-5600, emulator-5602, … (a porta de console). Escolha esse
aparelho. Não crie um AVD no Studio para isso.

Se a lista vier sem o emulator-*, ligue o aparelho depois deste script:
o emulador se anuncia ao adb quando sobe. Se ele já estava ligado, desligue
e ligue de novo.

Motor leve no Linux (Redroid): a porta começa em 5700.
  adb connect 127.0.0.1:5700

O Studio e o MultiDroid usam o mesmo servidor, na porta 5037.
Não defina ANDROID_ADB_SERVER_PORT.
EOF
