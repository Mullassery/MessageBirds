# pycustomerhub-android

Kotlin client SDK for PyCustomerHub — collects events on Android and streams
them to the CDP. Mirrors `sdk/web` and `sdk/ios` exactly in design: a
`localStorage`/file-backed queue, one real `POST /events` call per event,
exponential-backoff retry, drop after 5 attempts.

## Status: written, not yet build-verified

This module was written in an environment with no `gradle`, no `kotlinc`,
and no Android SDK installed. It targets real, standard Android/Kotlin APIs
(`HttpURLConnection`, `SharedPreferences`, `kotlinx.coroutines`), but unlike
`sdk/web` (vitest) and `sdk/ios` (`swift build` + `swift run pch-verify`),
it has not been compiled or unit-tested. See `docs/ARCHITECTURE.md`'s
Phase 6 section — this is a disclosed gap, not a claim of verified
correctness.

The Gradle wrapper (`gradlew`/`gradlew.bat`/`gradle/wrapper/gradle-wrapper.jar`)
is intentionally not committed — generating it correctly needs a working
Gradle install and network access this session didn't have. On a machine
with Gradle installed:

```bash
gradle wrapper --gradle-version 8.9
./gradlew build test
```

## Usage

```kotlin
val analytics = PyCustomerHubAnalytics(
    context = applicationContext,
    tenantId = "<tenant_id>",
    apiUrl = "https://your-pch-api-or-edge-gateway",
)
analytics.identify("anonymous_id", "anon-1")
analytics.track("commerce.product_view", data = JSONObject().put("product_id", "p1"))
```

## License

Apache-2.0. See the repo root [`LICENSE`](../../LICENSE).
