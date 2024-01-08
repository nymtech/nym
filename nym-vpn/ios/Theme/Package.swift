// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Theme",
    defaultLocalization: "en",
    platforms: [
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "Theme",
            targets: ["Theme"]
        )
    ],
    targets: [
        .target(
            name: "Theme",
            resources: [
                .copy("Resources/Fonts/Lato-Bold.ttf"),
                .copy("Resources/Fonts/Lato-Regular.ttf"),
                .copy("Resources/Fonts/Lato-SemiBold.ttf"),
                .process("Resources/Colors.xcassets"),
                .process("Resources/Localizable.xcstrings")
            ]
        ),
        .testTarget(
            name: "ThemeTests",
            dependencies: ["Theme"]
        )
    ]
)
