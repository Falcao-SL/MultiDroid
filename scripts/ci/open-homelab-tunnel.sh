#!/usr/bin/env bash
# The repository key is restricted server-side to two localhost forwards.
set -euo pipefail
: "${MULTIDROID_TUNNEL_SSH_KEY:?Missing tunnel key}"
: "${MULTIDROID_TUNNEL_HOST:?Missing tunnel host}"
: "${MULTIDROID_TUNNEL_HOST_KEY:?Missing pinned host key}"
: "${GITHUB_ENV:?Missing GitHub environment file}"
[[ "$MULTIDROID_TUNNEL_HOST" =~ ^[a-zA-Z0-9.-]+$ ]] || { echo 'Invalid tunnel host' >&2; exit 1; }
[[ "$MULTIDROID_TUNNEL_HOST_KEY" == ssh-ed25519\ * && "$MULTIDROID_TUNNEL_HOST_KEY" != *$'\n'* ]] || { echo 'Invalid pinned host key' >&2; exit 1; }
umask 077
work=$(mktemp -d /tmp/md-tunnel.XXXXXXXX)
printf 'MULTIDROID_TUNNEL_DIR=%s\n' "$work" >> "$GITHUB_ENV"
printf '%s\n' "$MULTIDROID_TUNNEL_SSH_KEY" > "$work/id_ed25519"
printf '%s %s\n' "$MULTIDROID_TUNNEL_HOST" "$MULTIDROID_TUNNEL_HOST_KEY" > "$work/known_hosts"
read -r phase_port minio_port < <(python3 - <<'PY'
import socket
sockets = []
for _ in range(2):
    sock = socket.socket()
    sock.bind(('127.0.0.1', 0))
    sockets.append(sock)
print(*(s.getsockname()[1] for s in sockets))
PY
)
ssh -i "$work/id_ed25519" -o IdentitiesOnly=yes -o BatchMode=yes \
  -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$work/known_hosts" \
  -o ExitOnForwardFailure=yes -o ConnectTimeout=15 -M -S "$work/control" -f -N \
  -L "127.0.0.1:$phase_port:127.0.0.1:8087" \
  -L "127.0.0.1:$minio_port:127.0.0.1:9000" \
  "multidroid-ci-tunnel@$MULTIDROID_TUNNEL_HOST"
printf 'PHASE_HOST=http://127.0.0.1:%s\n' "$phase_port" >> "$GITHUB_ENV"
printf 'MINIO_TUNNEL_ENDPOINT=http://127.0.0.1:%s\n' "$minio_port" >> "$GITHUB_ENV"
echo 'Restricted SSH tunnel established.'
