// swift-tools-version: 6.0
// Duo para Mac — vários aparelhos Android lado a lado (docs/spec/MAC.md).
// Pacote SwiftPM; scripts/build-app.sh monta o .app com Info.plist e ícone.
import PackageDescription

let package = Package(
    name: "Duo",
    platforms: [.macOS("26.0")],
    products: [
        .executable(name: "Duo", targets: ["Duo"]),
    ],
    dependencies: [
        .package(url: "https://github.com/grpc/grpc-swift-2.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-nio-transport.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-protobuf.git", from: "2.0.0"),
        .package(url: "https://github.com/apple/swift-protobuf.git", from: "1.28.0"),
    ],
    targets: [
        .executableTarget(
            name: "Duo",
            dependencies: [
                .product(name: "GRPCCore", package: "grpc-swift-2"),
                .product(name: "GRPCNIOTransportHTTP2", package: "grpc-swift-nio-transport"),
                .product(name: "GRPCProtobuf", package: "grpc-swift-protobuf"),
                .product(name: "SwiftProtobuf", package: "swift-protobuf"),
            ],
            path: "Sources/Duo"
        ),
    ]
)
