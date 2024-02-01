// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Settings",
    defaultLocalization: "en",
    platforms: [
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "Settings",
            targets: ["Settings"]
        )
    ],
    dependencies: [
//        .package(name: "AppVersionProvider", path: "../Services"),
//        .package(name: "AppSettings", path: "../Services"),
        .package(path: "../UIComponents")
    ],
    targets: [
        .target(
            name: "Settings",
            dependencies: [
//                "AppVersionProvider",
//                "AppSettings",
                "UIComponents"
            ]
        ),
        .testTarget(
            name: "SettingsTests",
            dependencies: ["Settings"]
        )
    ]
)
