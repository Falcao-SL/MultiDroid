#!/usr/bin/env bash
# Gera o cliente Swift da API gRPC do emulador a partir de proto/emulator_controller.proto
# (copiado de $ANDROID_SDK/emulator/lib/; atualize junto com a versão do emulador).
set -euo pipefail
cd "$(dirname "$0")/.."
swift build -c release --product protoc-gen-swift >/dev/null
swift build -c release --product protoc-gen-grpc-swift-2 >/dev/null
protoc -I proto \
  --plugin=protoc-gen-swift=.build/release/protoc-gen-swift \
  --plugin=protoc-gen-grpc-swift-2=.build/release/protoc-gen-grpc-swift-2 \
  --swift_out=Visibility=Internal:Sources/Duo/Generated \
  --grpc-swift-2_out=Client=true,Server=false,Visibility=Internal:Sources/Duo/Generated \
  proto/emulator_controller.proto
echo "gerado em Sources/Duo/Generated"
