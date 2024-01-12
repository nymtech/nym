// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Home",
    defaultLocalization: "en",
    platforms: [
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "Home",
            targets: ["Home"]
        )
    ],
    dependencies: [
        .package(path: "../UIComponents")
    ],
    targets: [
        .target(
            name: "Home",
            dependencies: ["UIComponents"],
            path: "Sources"
        ),
        .testTarget(
            name: "HomeTests",
            dependencies: ["Home"]
        )
    ]
)
