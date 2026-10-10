#!/usr/bin/env bash
# Fase 0 do modo leve do Duo para Mac (MAC.md §3, "Fase 0 do modo leve").
# Vários Androids em contêiner (Redroid) dentro de UMA VM Linux do
# Virtualization.framework, para medir quanto custa cada aparelho comparado ao
# emulador (~4,5 GB). A VM é criada pelo Lima com --vm-type=vz: é o mesmo
# hipervisor que o Duo usaria embutido, então os números valem para o app.
#
#   mac/tools/leve.sh setup                 cria a VM, instala binder e Docker, baixa a imagem
#   mac/tools/leve.sh up <N> [ksm]          liga N Androids (ksm: liga a fusão de páginas)
#   mac/tools/leve.sh medir                 uma linha com memória do Mac, da VM e de cada Android
#   mac/tools/leve.sh rolar <i>             rola Configurações no Android i e mede quadros com atraso
#   mac/tools/leve.sh video <i>             toca um vídeo de feed no Android i e conta erros do decodificador
#   mac/tools/leve.sh tela <i>              mostra o Android i com o scrcpy (brew install scrcpy)
#   mac/tools/leve.sh down                  desliga os Androids (a VM continua)
#   mac/tools/leve.sh reiniciar             reinicia a VM, para cada rodada começar do zero
#   mac/tools/leve.sh destruir              apaga a VM
#
# Exemplo de rodada: setup; reiniciar; medir; up 1; medir; reiniciar; up 4; medir; rolar 1; reiniciar; up 4 ksm; medir
set -u
VM=duo-leve
IMG=${DUO_LEVE_IMG:-redroid/redroid:13.0.0_64only-latest}
W=${DUO_LEVE_W:-720} H=${DUO_LEVE_H:-1600} DPI=${DUO_LEVE_DPI:-320}
# DUO_LEVE_EXTRA: parâmetros androidboot.* a mais, para experimentar (ex.: gralloc)
PORTA0=5560                      # Android i escuta o adb em 127.0.0.1:(PORTA0+i)
ADB=${ADB:-$HOME/Library/Android/sdk/platform-tools/adb}

vm() { limactl shell "$VM" -- "$@"; }
sdo() { vm sudo "$@"; }

case "${1:-}" in
setup)
  command -v limactl >/dev/null || brew install lima
  limactl list -q 2>/dev/null | grep -qx "$VM" || \
    limactl create -y --name="$VM" --vm-type=vz --cpus=6 --memory=12 --disk=40 \
      --mount-none --containerd=none template:ubuntu-24.04
  limactl start -y "$VM" >/dev/null
  # binder: o "correio" entre processos do Android. Vem como módulo no kernel
  # genérico do Ubuntu, no pacote extra. binderfs é montado pelo próprio Redroid.
  vm bash -c 'sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq &&
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq linux-modules-extra-$(uname -r) docker.io gcc >/dev/null &&
    echo binder_linux | sudo tee /etc/modules-load.d/duo-binder.conf >/dev/null &&
    echo "options binder_linux devices=binder,hwbinder,vndbinder" | sudo tee /etc/modprobe.d/duo-binder.conf >/dev/null &&
    sudo modprobe binder_linux'
  # ksm-exec: liga PR_SET_MEMORY_MERGE (Linux 6.4+) e executa o /init do Android.
  # A marca passa para os filhos, então o Android inteiro fica elegível para o KSM
  # fundir páginas iguais entre aparelhos sem ninguém chamar madvise.
  vm bash -c 'cat > /tmp/ksm-exec.c <<EOF
#include <sys/prctl.h>
#include <unistd.h>
#include <stdio.h>
#ifndef PR_SET_MEMORY_MERGE
#define PR_SET_MEMORY_MERGE 67
#endif
int main(int c, char **v) {
  if (prctl(PR_SET_MEMORY_MERGE, 1, 0, 0, 0)) perror("PR_SET_MEMORY_MERGE");
  execv(v[1], v + 1); perror("execv"); return 1;
}
EOF
    sudo mkdir -p /opt/duo && sudo gcc -static -O2 -o /opt/duo/ksm-exec /tmp/ksm-exec.c'
  sdo docker pull -q "$IMG"
  echo "pronto: $(vm uname -r), binder $(vm lsmod | grep -c binder_linux), imagem $IMG"
  ;;

up)
  N=${2:?quantos Androids}; KSM=${3:-}
  if [ "$KSM" = ksm ]; then
    sdo sh -c 'echo 1 > /sys/kernel/mm/ksm/run; echo 1000 > /sys/kernel/mm/ksm/pages_to_scan; echo 20 > /sys/kernel/mm/ksm/sleep_millisecs'
  else
    sdo sh -c 'echo 2 > /sys/kernel/mm/ksm/run'   # 2 = para e desfaz as fusões
  fi
  for i in $(seq 1 "$N"); do
    entry=(); [ "$KSM" = ksm ] && entry=(-v /opt/duo/ksm-exec:/ksm-exec:ro --entrypoint /ksm-exec)
    args=(androidboot.redroid_width=$W androidboot.redroid_height=$H androidboot.redroid_dpi=$DPI
          androidboot.redroid_fps=30 androidboot.redroid_gpu_mode=guest
          androidboot.use_memfd=true ${DUO_LEVE_EXTRA:-})   # memfd: o kernel 6.8 não tem ashmem
    [ "$KSM" = ksm ] && args=(/init qemu=1 androidboot.hardware=redroid "${args[@]}")
    sdo docker rm -fv duo-leve-$i >/dev/null 2>&1
    t0=$(date +%s)
    sdo docker run -d --privileged --name duo-leve-$i -p $((PORTA0+i)):5555 \
      ${entry[@]+"${entry[@]}"} "$IMG" "${args[@]}" >/dev/null || exit 1
    for _ in $(seq 1 90); do
      [ "$(sdo docker exec duo-leve-$i getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ] && break
      sleep 2
    done
    echo "Android $i ligou em $(( $(date +%s) - t0 ))s (adb em 127.0.0.1:$((PORTA0+i)))"
  done
  ;;

medir)
  sleep "${ESPERA:-45}"     # deixa o boot assentar, como no measure-image.sh
  N=$(sdo docker ps -q --filter name=duo-leve- | wc -l | tr -d ' ')
  # O lado do Mac: o processo da VM do Virtualization.framework.
  pid=$(pgrep -f com.apple.Virtualization.VirtualMachine | head -1)
  foot=$(footprint -p "$pid" 2>/dev/null | awk '/Footprint:/{for(k=1;k<NF;k++) if($k=="Footprint:"){print $(k+1),$(k+2); exit}}')
  [ -n "$foot" ] || foot=$(vmmap --summary "$pid" 2>/dev/null | awk -F: '/Physical footprint:/{gsub(/ /,"",$2);print $2}')
  # O lado da VM: memória usada de fato e cache (a imagem compartilhada mora no cache).
  read -r usada cache < <(vm free -m | awk '/Mem:/{print $3, $6}')
  ksm=$(vm cat /sys/kernel/mm/ksm/pages_sharing /sys/kernel/mm/ksm/pages_shared | paste -sd' ' - |
        awk '{printf "%.0f MB economizados", ($1)*4/1024}')
  por=$(sdo docker stats --no-stream --format '{{.Name}}={{.MemUsage}}' | sed 's/ \/.*//' | sort | paste -sd' ' -)
  echo "N=$N | Mac: VM com footprint $foot | VM: usada ${usada} MB, cache ${cache} MB | KSM: $ksm | $por"
  ;;

rolar)
  i=${2:-1}; c=duo-leve-$i
  sdo docker exec $c sh -c 'am start -W -a android.settings.SETTINGS >/dev/null; sleep 2;
    dumpsys gfxinfo com.android.settings reset >/dev/null
    for k in 1 2 3 4 5 6 7 8 9 10; do input swipe 360 1300 360 300 200; input swipe 360 300 360 1300 200; done
    dumpsys gfxinfo com.android.settings' |
    grep -E 'Total frames rendered|Janky frames:|50th percentile|90th percentile|99th percentile' | sed 's/^ *//'
  ;;

video)   # toca um vídeo vertical H.264 (feed) e diz se o decodificador deu conta
  i=${2:-1}; s=127.0.0.1:$((PORTA0+i)); dir=${TMPDIR:-/tmp}/duo-leve; mkdir -p "$dir"
  [ -f "$dir/feed.mp4" ] || ffmpeg -loglevel error -y -f lavfi -i "testsrc2=size=720x1280:rate=30,format=yuv420p" \
    -f lavfi -i "sine=frequency=440" -t 60 -c:v libx264 -profile:v high -b:v 3M -c:a aac -shortest "$dir/feed.mp4"
  "$ADB" connect $s >/dev/null
  "$ADB" -s $s logcat -c
  "$ADB" -s $s push -q "$dir/feed.mp4" /data/local/tmp/feed.mp4
  "$ADB" -s $s shell 'chmod 644 /data/local/tmp/feed.mp4; settings put secure immersive_mode_confirmations confirmed
    am start -W -a android.intent.action.VIEW -d file:///data/local/tmp/feed.mp4 -t video/mp4' >/dev/null
  sleep 12
  "$ADB" -s $s exec-out screencap -p > "$dir/video-$i.png"
  erros=$("$ADB" -s $s logcat -d | grep -cE 'graphic view map failed|NuPlayer.*error')
  cpu=$("$ADB" -s $s shell top -b -n1 | awk '/%cpu/{print $2, $4; exit}')
  echo "Android $i: $(sdo docker inspect -f '{{.Config.Image}}' duo-leve-$i) | erros do decodificador: $erros | CPU do Android: ${cpu:-?} | tela: $dir/video-$i.png"
  ;;

tela)
  i=${2:-1}
  command -v scrcpy >/dev/null || { echo "instale: brew install scrcpy"; exit 1; }
  "$ADB" connect 127.0.0.1:$((PORTA0+i)) >/dev/null
  ADB="$ADB" scrcpy -s 127.0.0.1:$((PORTA0+i)) --window-title "Duo leve $i"
  ;;

down)
  sdo sh -c 'docker rm -fv $(docker ps -aq --filter name=duo-leve-) 2>/dev/null; echo 2 > /sys/kernel/mm/ksm/run' >/dev/null
  echo "Androids desligados; a VM continua ligada (leve.sh destruir apaga)"
  ;;

destruir)
  limactl delete -f "$VM"
  ;;

reiniciar)   # zera o footprint no Mac: a VM não devolve ao macOS a memória que já tocou
  sdo sh -c 'docker rm -fv $(docker ps -aq --filter name=duo-leve-) 2>/dev/null' >/dev/null
  limactl stop "$VM" >/dev/null 2>&1; limactl start -y "$VM" >/dev/null 2>&1 && echo "VM reiniciada"
  ;;

*)
  sed -n '2,22p' "$0"; exit 1 ;;
esac
