#!/usr/bin/env bash
# Fase 0 do MultiDroid: mede o custo real de uma imagem de sistema.
# Cria um AVD, liga a frio sem janela, mede o boot e a memória com o aparelho
# parado, e desliga. Um aparelho por vez, para a medida não se misturar.
#
#   mac/tools/measure-image.sh <nome> "<pacote da imagem>" <perfil> <RAM MB> <núcleos>
#   mac/tools/measure-image.sh t36-play "system-images;android-36;google_apis_playstore;arm64-v8a" pixel_9 2048 2
set -u
NAME=$1; IMAGE=$2; DEVICE=$3; RAM=$4; CORES=$5
SDK=${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}
ADB=$SDK/platform-tools/adb
PORT=5570
LOG=/tmp/multidroid-measure-$NAME.log

echo no | "$SDK/cmdline-tools/latest/bin/avdmanager" create avd -n "$NAME" -k "$IMAGE" -d "$DEVICE" --force >/dev/null 2>&1
t0=$(date +%s)
"$SDK/emulator/emulator" -avd "$NAME" -memory "$RAM" -cores "$CORES" -no-window -no-audio -no-boot-anim \
  -gpu host -no-snapshot -port $PORT >"$LOG" 2>&1 &
booted=""
for _ in $(seq 1 150); do
  [ "$("$ADB" -s emulator-$PORT shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ] && { booted=1; break; }
  sleep 2
done
boot=$(( $(date +%s) - t0 ))
forced=$(grep -o 'Increasing RAM size to [0-9]*MB' "$LOG" | head -1)
if [ -n "$booted" ]; then
  sleep 45
  pid=$(pgrep -f "qemu-system-aarch64.*-avd $NAME " | head -1)
  foot=$(vmmap --summary "$pid" 2>/dev/null | awk -F: '/Physical footprint:/{gsub(/ /,"",$2);print $2}')
  cpu=$(ps -o %cpu= -p "$pid" | tr -d ' ')
  guest=$("$ADB" -s emulator-$PORT shell cat /proc/meminfo | awk '/MemTotal/{printf "%.1f GB",$2/1048576}')
  avail=$("$ADB" -s emulator-$PORT shell cat /proc/meminfo | awk '/MemAvailable/{printf "%.1f GB",$2/1048576}')
  echo "$NAME | ${boot}s | RAM do Android $guest (livre $avail) | footprint $foot | CPU parado ${cpu}% | ${forced:-sem RAM forçada}"
else
  echo "$NAME | NÃO LIGOU em ${boot}s | ${forced:-sem RAM forçada} | log: $LOG"
fi
"$ADB" -s emulator-$PORT emu kill >/dev/null 2>&1
sleep 5
pkill -f "qemu-system-aarch64.*-avd $NAME " 2>/dev/null
sleep 2
