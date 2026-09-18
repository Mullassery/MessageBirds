// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "MessageBirdsAnalytics",
    platforms: [
        .iOS(.v15),
        .macOS(.v12),
    ],
    products: [
        .library(name: "MessageBirdsAnalytics", targets: ["MessageBirdsAnalytics"])
    ],
    targets: [
        .target(name: "MessageBirdsAnalytics", dependencies: []),
        .testTarget(name: "MessageBirdsAnalyticsTests", dependencies: ["MessageBirdsAnalytics"]),
        // `swift test` needs XCTest/swift-testing, neither of which is
        // available without full Xcode.app (this environment only has the
        // Command Line Tools — confirmed `xcrun --find xctest` fails and
        // `import Testing` doesn't resolve under `swift test`). This
        // executable exercises the same EventQueue scenarios as
        // MessageBirdsAnalyticsTests with plain `assert`s, so the logic is
        // still genuinely verified here via `swift run mb-verify` even
        // though the XCTest suite itself can only run in CI/Xcode.
        .executableTarget(name: "mb-verify", dependencies: ["MessageBirdsAnalytics"], path: "Sources/VerifyCLI"),
    ]
)
