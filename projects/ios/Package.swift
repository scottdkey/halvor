// swift-tools-version: 6.0
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "HalvorSwift",
    platforms: [
        .iOS(.v18),
        .macOS(.v15),
    ],
    products: [
        .library(
            name: "HalvorSwift",
            targets: ["HalvorSwift"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "HalvorSwift",
            dependencies: ["HalvorSwiftFFI"],
            path: "Sources/HalvorSwift"
        ),
        .target(
            name: "HalvorSwiftFFI",
            dependencies: ["HalvorSwiftFFIBinary"],
            path: "Sources/HalvorSwiftFFI/halvor_ffi",
            sources: ["halvor_ffi.swift", "generated_swift_bindings.swift"],
            linkerSettings: [
                .linkedFramework("SystemConfiguration", .when(platforms: [.macOS, .iOS])),
            ]
        ),
        .binaryTarget(
            name: "HalvorSwiftFFIBinary",
            path: "Sources/HalvorSwiftFFI/HalvorSwiftFFI.xcframework"
        ),
        .executableTarget(
            name: "BasicExample",
            dependencies: ["HalvorSwift"],
            path: "Examples",
            exclude: ["AsyncExample.swift"],
            sources: ["BasicExample.swift"]
        ),
        .executableTarget(
            name: "AsyncExample",
            dependencies: ["HalvorSwift"],
            path: "Examples",
            exclude: ["BasicExample.swift"],
            sources: ["AsyncExample.swift"]
        ),
        .executableTarget(
            name: "HalvorApp",
            dependencies: ["HalvorSwift"],
            path: "Sources/HalvorApp",
            exclude: ["Info-iOS.plist", "Info-macOS.plist"],
            sources: ["HalvorApp.swift", "ContentView.swift"]
        ),
    ]
)
