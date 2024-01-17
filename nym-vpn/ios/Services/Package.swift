// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Services",
    defaultLocalization: "en",
    platforms: [
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "Tunnels",
            targets: ["Tunnels"]
        )
    ],
    targets: [
        .target(
            name: "Tunnels",
            dependencies: [],
            path: "Sources/Services/Tunnels"
        ),
        .testTarget(
            name: "TunnelsTests",
            dependencies: ["Tunnels"]
        )
    ]
)
