#!/usr/bin/env bash
# Emulador enxuto: quanto cada ajuste tira da memória de um celular sem perder
# o que importa (rolagem fluida e vídeo tocando). Um aparelho por vez, boot a
# frio, no SDK do próprio MultiDroid; o aparelho de teste vive numa pasta temporária.
#
#   mac/tools/measure-lean.sh <nome> <gpu> <largura>x<altura> <dpi> <RAM MB>
#   mac/tools/measure-lean.sh base host 1080x2424 420 2048
#   mac/tools/measure-lean.sh sw   swiftshader 1080x2424 420 2048
#
# gpu: host | swiftshader | swangle | lavapipe | software (emulator -help-gpu)
set -u
NAME=$1; GPU=$2; RES=$3; DPI=$4; RAM=$5
W=${RES%x*}; H=${RES#*x}
MD="$HOME/Library/Application Support/MultiDroid"
SDK="$MD/sdk"
ADB=${ADB:-$HOME/Library/Android/sdk/platform-tools/adb}
WORK=${TMPDIR:-/tmp}/multidroid-lean
AVD=$WORK/avd
PORT=5570
S=emulator-$PORT
mkdir -p "$AVD"

# Vídeo vertical de feed (H.264, 30 fps, com movimento), gerado uma vez.
[ -f "$WORK/feed.mp4" ] || ffmpeg -loglevel error -y -f lavfi -i "testsrc2=size=720x1280:rate=30,format=yuv420p" \
  -t 60 -c:v libx264 -profile:v high -b:v 3M "$WORK/feed.mp4"

# Aparelho de teste: os mesmos campos que o Catalog.config do app grava para o celular.
rm -rf "$AVD/$NAME.avd" "$AVD/$NAME.ini"
mkdir -p "$AVD/$NAME.avd"
printf 'avd.ini.encoding=UTF-8\npath=%s\ntarget=android-33\n' "$AVD/$NAME.avd" > "$AVD/$NAME.ini"
cat > "$AVD/$NAME.avd/config.ini" <<EOF
avd.ini.encoding=UTF-8
AvdId=$NAME
abi.type=arm64-v8a
hw.cpu.arch=arm64
hw.cpu.ncore=2
hw.ramSize=$RAM
vm.heapSize=228M
hw.device.name=pixel_9
hw.lcd.width=$W
hw.lcd.height=$H
hw.lcd.density=$DPI
hw.lcd.depth=32
hw.keyboard=yes
hw.mainKeys=no
hw.gpu.enabled=yes
hw.gpu.mode=$GPU
hw.audioInput=no
hw.audioOutput=no
hw.sdCard=no
hw.camera.back=none
hw.camera.front=none
disk.dataPartition.size=6G
image.sysdir.1=system-images/android-33/google_apis_playstore/arm64-v8a/
tag.id=google_apis_playstore
PlayStore.enabled=no
EOF

export ANDROID_SDK_ROOT="$SDK" ANDROID_AVD_HOME="$AVD" ANDROID_EMULATOR_HOME="$WORK"
LOG=$WORK/$NAME.log
t0=$(date +%s)
"$SDK/emulator/emulator" -avd "$NAME" -no-window -no-audio -no-boot-anim -no-metrics -gpu "$GPU" \
  -no-snapshot -port $PORT >"$LOG" 2>&1 &
"$ADB" start-server >/dev/null 2>&1
booted=""
for _ in $(seq 1 150); do
  [ "$("$ADB" -s $S shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ] && { booted=1; break; }
  sleep 2
done
boot=$(( $(date +%s) - t0 ))
pid=$(pgrep -f "qemu-system-aarch64.*-avd $NAME( |$)" | head -1)
# Memória: o "Physical footprint" do vmmap (o mesmo do Monitor de Atividade e da
# fase 0), que conta a memória comprimida pelo tamanho original; entre
# parênteses, o residente sem a comprimida (footprint).
foot() {
  v=$(vmmap --summary "$pid" 2>/dev/null | awk -F: '/Physical footprint:/{gsub(/ /,"",$2);print $2; exit}')
  r=$(footprint -p "$pid" 2>/dev/null | awk '/Footprint:/{for(k=1;k<NF;k++) if($k=="Footprint:"){print $(k+1)$(k+2); exit}}')
  echo "$v ($r residente)"
}
cpu() { ps -o %cpu= -p "$pid" | tr -d ' '; }

if [ -z "$booted" ]; then
  echo "$NAME | gpu=$GPU ${W}x$H@$DPI RAM=$RAM | NÃO LIGOU em ${boot}s | log: $LOG"
else
  sleep ${ESPERA:-60}
  parado="$(foot) / CPU $(cpu)%"
  forced=$(grep -o 'Increasing RAM size to [0-9]*MB' "$LOG" | head -1)

  # Rolagem: 20 rolagens em Configurações; quadros com atraso pelo gfxinfo.
  # Coordenadas proporcionais à tela: com valores fixos, a tela menor não rolava.
  x=$((W/2)); y1=$((H*8/10)); y2=$((H*2/10))
  "$ADB" -s $S shell "am start -W -a android.settings.SETTINGS >/dev/null; sleep 2
    dumpsys gfxinfo com.android.settings reset >/dev/null
    for k in 1 2 3 4 5 6 7 8 9 10; do input swipe $x $y1 $x $y2 200; input swipe $x $y2 $x $y1 200; done" >/dev/null
  rol=$("$ADB" -s $S shell dumpsys gfxinfo com.android.settings |
        awk -F': ' '/Janky frames:/{j=$2} /50th percentile/{m=$2} END{print j" com atraso, mediana "m}')

  # Vídeo: toca o feed e compara duas capturas com 1 s de diferença (vídeo andando = diferentes).
  "$ADB" -s $S push -q "$WORK/feed.mp4" /data/local/tmp/feed.mp4
  "$ADB" -s $S shell 'chmod 644 /data/local/tmp/feed.mp4; settings put secure immersive_mode_confirmations confirmed
    am start -a android.intent.action.VIEW -d file:///data/local/tmp/feed.mp4 -t video/mp4' >/dev/null 2>&1
  sleep 6
  a=$("$ADB" -s $S exec-out screencap | md5); sleep 1; b=$("$ADB" -s $S exec-out screencap | md5)
  quem=$("$ADB" -s $S shell dumpsys window | awk '/mCurrentFocus/{print $NF; exit}' | tr -d '}\r')
  if [ "$a" != "$b" ]; then video="toca"; else video="parado ($quem)"; fi
  tocando="$(foot) / CPU $(cpu)%"
  "$ADB" -s $S exec-out screencap -p > "$WORK/$NAME-video.png"

  echo "$NAME | gpu=$GPU ${W}x$H@$DPI RAM=$RAM | boot ${boot}s | parado: $parado | rolagem: $rol | vídeo: $video, $tocando | ${forced:-RAM aceita}"
fi
"$ADB" -s $S emu kill >/dev/null 2>&1
sleep 5
pkill -f "qemu-system-aarch64.*-avd $NAME( |$)" 2>/dev/null
sleep 2
rm -rf "$AVD/$NAME.avd" "$AVD/$NAME.ini"
