// swift-tools-version: 6.0
// MultiDroid — várias contas e aparelhos Android ao mesmo tempo, no Mac (docs/spec/MAC.md).
// Pacote SwiftPM; scripts/build-app.sh monta o .app com Info.plist e ícone.
import PackageDescription

let package = Package(
    name: "MultiDroid",
    platforms: [.macOS("26.0")],
    products: [
        .executable(name: "MultiDroid", targets: ["MultiDroid"]),
    ],
    dependencies: [
        .package(url: "https://github.com/grpc/grpc-swift-2.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-nio-transport.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-protobuf.git", from: "2.0.0"),
        .package(url: "https://github.com/apple/swift-protobuf.git", from: "1.28.0"),
    ],
    targets: [
        .executableTarget(
            name: "MultiDroid",
            dependencies: [
                .product(name: "GRPCCore", package: "grpc-swift-2"),
                .product(name: "GRPCNIOTransportHTTP2", package: "grpc-swift-nio-transport"),
                .product(name: "GRPCProtobuf", package: "grpc-swift-protobuf"),
                .product(name: "SwiftProtobuf", package: "swift-protobuf"),
            ],
            path: "Sources/MultiDroid"
        ),
    ]
)
