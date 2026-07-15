# SlimeRelay Android Implementation

## Files to place in `SlimeRelay/app/src/main/java/com/github/yfby/slimerelay/`

| File | Purpose |
|------|---------|
| `SlimeRelayConstants.kt` | Protocol constants matching `lib.rs` |
| `UdpProtocol.kt` | UDP handshake + f32 conversion, mirrors `net.rs` |
| `AudioService.kt` | AudioRecord (server) and AudioTrack (client) wrappers |
| `RelayUiState.kt` | UI state data class |
| `RelayService.kt` | Foreground service with UDP loop |
| `MainActivity.kt` | Compose UI with mode toggle, IP input, connect button |

## Files to modify

| File | Changes |
|------|---------|
| `app/src/main/AndroidManifest.xml` | Add FOREGROUND_SERVICE permissions + service declaration |
| `app/build.gradle.kts` | Add lifecycle-viewmodel-compose dependency |
| `gradle/libs.versions.toml` | Add lifecycleViewmodelCompose version |

## Architecture

```
MainActivity (Compose UI)
  ↓ binds to
RelayService (Foreground Service)
  ├── Client mode: UDP recv → SpeakerPlayer
  └── Server mode: MicRecorder → UDP send
```

Wire-compatible with the Rust desktop app.
