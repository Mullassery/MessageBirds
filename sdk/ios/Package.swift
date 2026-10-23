// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "PyCustomerHubAnalytics",
    platforms: [
        .iOS(.v15),
        .macOS(.v12),
    ],
    products: [
        .library(name: "PyCustomerHubAnalytics", targets: ["PyCustomerHubAnalytics"])
    ],
    targets: [
        .target(name: "PyCustomerHubAnalytics", dependencies: []),
        .testTarget(name: "PyCustomerHubAnalyticsTests", dependencies: ["PyCustomerHubAnalytics"]),
        // `swift test` needs XCTest/swift-testing, neither of which is
        // available without full Xcode.app (this environment only has the
        // Command Line Tools — confirmed `xcrun --find xctest` fails and
        // `import Testing` doesn't resolve under `swift test`). This
        // executable exercises the same EventQueue scenarios as
        // PyCustomerHubAnalyticsTests with plain `assert`s, so the logic is
        // still genuinely verified here via `swift run pch-verify` even
        // though the XCTest suite itself can only run in CI/Xcode.
        .executableTarget(name: "pch-verify", dependencies: ["PyCustomerHubAnalytics"], path: "Sources/VerifyCLI"),
    ]
)
