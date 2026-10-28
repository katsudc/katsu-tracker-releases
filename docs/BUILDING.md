# Building from source

## Windows requirements

- Windows x64 and the WebView2 runtime.
- Node.js 22 LTS or newer and pnpm 11.24.0 (pinned in `package.json`).
- Stable Rust with the `x86_64-pc-windows-msvc` target.
- Visual Studio Build Tools: Desktop development with C++, MSVC and the Windows SDK.

## Develop and validate

```powershell
pnpm install --frozen-lockfile
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
pnpm exec tauri dev
```

Some Rust tests marked `ignored` need a running game or media session and are opt-in. Regular tests use local fixtures. `--riot-offline-diagnostic` disables Riot session/API traffic for troubleshooting a built application.

## Local executable or installer

```powershell
# Executable without an installer or publisher signing credentials:
pnpm exec tauri build --no-bundle -- --locked

# Local installer without official updater-signature artifacts:
pnpm exec tauri build --config src-tauri/tauri.dev-build.json --bundles nsis -- --locked
```

The executable is produced under `src-tauri/target/release/`; the installer is under `src-tauri/target/release/bundle/nsis/`. Local builds are not guaranteed to be bit-for-bit identical to a published binary: toolchain, environment and signing can change the output.

## Architecture

| Area | Location |
| --- | --- |
| React UI, desktop layout and settings | `src/components`, `src/stores` |
| Translations | `src/lib/i18n.ts` |
| Session and game-data client | `src-tauri/src/api` |
| Personal match history | `src-tauri/src/tracker`, `src/tracker` |
| Visual Spike detector and round state | `src-tauri/src/spike*.rs` |
| Overlay appearance and native placement | `spike_appearance.rs`, `spike_display.rs`, `SpikeOverlayEditor.tsx` |
| Signed update discovery and download | `src-tauri/src/updates.rs` |
| Independent silent installer progress process | `src-tauri/src/update_worker.rs` |
| Spotify desktop media integration | `src-tauri/src/media.rs` |

The editor uses a user-provided HUD screenshot over a black background, displayed at reduced opacity. The draggable timer is a separate, full-opacity element. This is a static preview, not a live game capture. Overlay appearance is independent of icon recognition. Do not change the working detector ROI, thresholds, capture, polling or round timing as part of display/layout work. External overlays depend on Windows desktop composition; true exclusive fullscreen visibility is not guaranteed. See [Microsoft's Fullscreen Optimizations explanation](https://devblogs.microsoft.com/directx/demystifying-full-screen-optimizations/).

## Official releases and forks

`scripts/build_update.ps1` and `scripts/publish_update.ps1` are maintainer release tools. The official updater public key and release endpoint are intentionally public. Private signing keys and the DPAPI-protected signing password are stored outside this repository and are never required for local builds.

Use your own application identifier, release endpoint and signing key pair if distributing an independent fork. Keep the official identifier unchanged only when intentionally upgrading an existing official installation. App preferences, Riot session data, local caches, credentials and signing material must not be committed.

The updater verifies the release signature before handing a package to the worker, verifies it again in the worker, waits for the previous process to exit, runs NSIS with `/S`, and confirms the restarted version through a one-time local acknowledgement. Download percentage is based on received bytes; installation is shown as a stage, not an invented percentage.

## Third-party code and test data

`src-tauri/vendor/tauri-winres` is a patched build dependency with its own MIT license and attribution. Keep that license when redistributing it. `src-tauri/tests/fixtures/localhost-test-only.p12` is a self-signed localhost TLS test identity with the public test password `test-only`; it is not an application, account or release credential.

GitHub calculates the repository language chart automatically. Vendored dependency code and generated test fixtures are excluded using `.gitattributes`.
