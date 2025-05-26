// swift-tools-version:5.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "tauri-plugin-cache",
    platforms: [
        .macOS(.v10_13),
        .iOS(.v13),
    ],
    products: [
        // Products define the executables and libraries a package produces, and make them visible to other packages.
        .library(
            name: "tauri-plugin-cache",
            type: .static,
            targets: ["tauri-plugin-cache"]),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api"),
        .package(url: "https://github.com/OlehKulykov/PLzmaSDK.git", .exact("1.5.0"))
    ],
    targets: [
        // Targets are the basic building blocks of a package. A target can define a module or a test suite.
        // Targets can depend on other targets in this package, and on products in packages this package depends on.
        .target(
            name: "tauri-plugin-cache",
            dependencies: [
                .byName(name: "Tauri"),
                .product(name: "LZMA", package: "PLzmaSDK")
            ],
            path: "Sources")
    ]
)