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
            name: "AppSettings",
            targets: ["AppSettings"]
        ),
        .library(
            name: "AppVersionProvider",
            targets: ["AppVersionProvider"]
        ),
        .library(
            name: "Modifiers",
            targets: ["Modifiers"]
        ),
        .library(
            name: "Tunnels",
            targets: ["Tunnels"]
        )
    ],
    targets: [
        .target(
            name: "AppSettings",
            dependencies: [],
            path: "Sources/Services/AppSettings"
        ),
        .target(
            name: "AppVersionProvider",
            dependencies: [],
            path: "Sources/Services/AppVersionProvider"
        ),
        .target(
            name: "Modifiers",
            dependencies: [
                "AppSettings"
            ],
            path: "Sources/Services/Modifiers"
        ),
        .target(
            name: "Tunnels",
            dependencies: [],
            path: "Sources/Services/Tunnels"
        )
    ]
)
