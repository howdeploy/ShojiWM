# Output overlay regression checks

## Compact checks for the local extension

Passed on 2026-10-02: seven SDK tests, the type fixture and both examples,
three native control tests, one cache-key test and the offscreen GPU test.
Native tests used Rust 1.94.0, offline Cargo and private IPC; every run preserved
the sentinel socket. No live QML files or user configuration were loaded:

```sh
node --import tsx tests/output-overlay.test.ts
npx tsc -p tests/tsconfig.overlays.json
python3 tests/run-native-tests.py cargo +1.94.0 test --offline -p shoji_wm backend::overlay::tests -- --test-threads=1
python3 tests/run-native-tests.py cargo +1.94.0 test --offline -p shoji_wm stable_lower_key_keeps_state -- --test-threads=1
```

This is seven SDK tests, one type fixture (plus the two examples), three native
control tests and one cache-key test. They reuse the existing runners and cover:

- Temporary deadline/default behavior and persistent first-frame readiness.
- Signal updates, failure cleanup, idempotent disposal and callback deadlock protection.
- Legacy background/window/layer/popup handlers during a persistent overlay and
  after full effect-config replacement, without mutating the plugin's object.
- Pending timeout in both modes, a ready temporary overlay still expiring, a ready
  persistent one surviving that deadline, disposal and runtime-owner teardown.
- The actual lower-layer key shared by TTY/Winit and its compatibility with
  cache eviction: other layers/outputs are isolated, a size change is a variant.

One optional offscreen GPU test below checks real native capture/placement,
live updates without self-feedback, reuse on an idle scene, two-output isolation,
output geometry/removal and lock cleanup. Its SDK/native portion runs the actual
pixelation shader at size 1 on a nonuniform image at scale 1.5, comparing the
complete output and final composition. It also verifies native persistent mode
and the absence of a frozen-scene allocation.

These contracts do not prove that every installed QML widget or third-party plugin
loads or looks correct in a real session; that still needs a separate user check.

The local checkout had no Node dependencies. This run used the existing TypeScript
5.9.3 from `agency-website/node_modules` and esbuild 0.28.1 from
`video-montage/node_modules` to bundle the unchanged SDK tests, then ran that bundle
with Node 26.8.1. No dependencies were installed and no package lock was changed.
The initial GPU failure was a test-fixture asset-root error: its temporary config
now has its own `package.json`, so the real pixelation shader resolves correctly.
Only a test binary was compiled; release build, installation, reload and session
checks were not performed.

## Existing broader checks

From the repository root, with its Node dev dependencies installed:

```sh
node --import tsx tests/output-overlay.test.ts
npx tsc -p packages/shoji_wm/tsconfig.json
npx tsc -p packages/config/tsconfig.json
npx tsc -p tests/tsconfig.overlays.json
python3 tests/run-native-tests.py cargo +1.94.0 test --offline -p shoji_wm -- --test-threads=1
cargo +1.94.0 build --offline --release -p shoji_wm
```

The package declares TypeScript 5.9. If using TypeScript 6, add
`--ignoreDeprecations 6.0` to tolerate its warning about the existing `baseUrl`.
The SDK tests use Node's built-in assertions/test runner; no test framework is added.

The persistent extension checks passed in the compact run above. They cover SDK
lifetime forwarding/reactive disposal, first-frame timeout in both modes and a
ready persistent overlay surviving its original deadline until disposal. The
existing offscreen overlay probe now uses persistent mode for its live-source
alpha check, which also checks that the slot excludes its own prior output.
The example typecheck includes both dissolve and pixelation files.

Explicit offscreen checks (no compositor session or visible window):

```sh
LIBGL_ALWAYS_SOFTWARE=1 EGL_PLATFORM=surfaceless python3 tests/run-native-tests.py cargo +1.94.0 test --offline -p shoji_wm output_overlay_gpu -- --ignored --nocapture --test-threads=1
LIBGL_ALWAYS_SOFTWARE=1 EGL_PLATFORM=surfaceless python3 tests/run-native-tests.py cargo +1.94.0 test --offline -p shoji_wm shader_reload_deletes_retired_programs_in_gl -- --ignored --nocapture --test-threads=1
```

These require a surfaceless EGL/OpenGL ES implementation. The overlay test fails
if one is unavailable; it does not silently skip. The default suite needs permission
to create Unix sockets. Rusty V8's prebuilt archive must already be cached for a
fully offline build; `RUSTY_V8_ARCHIVE` may point to the matching local library.

Never escalate a test run with the desktop's original IPC environment. The runner
uses a private runtime directory and display name, removes the inherited display
and wake PID, and verifies that its live sentinel socket was not replaced. Each
embedded test runtime also receives its own private IPC directory, removed at
teardown. Two default-config runtimes must coexist. The IPC regression checks both
recovery of an abandoned socket and rejection of a duplicate live server.

Review found and corrected a non-Clone render-element cache, backwards-incompatible
effect-config typing, capture waits that could block rendering, deadlines relying
solely on compositor ticks, and missing live-source opacity/transform invalidation.

The SDK checks include the actual output name, placement, deadline and descriptor
passed to the native bridge. The EGL probe additionally drives the real SDK and
native bridge through successful capture and disposal, and compares a complete
nonuniform image at scale 1.5, including the final composed scene. This catches
cropping/scaling bugs that the original single-pixel, solid-color checks missed.

The previous claim that the unrestricted suite left the live session untouched
was incorrect: loading the default config could replace its IPC socket. Use the
isolated runner above, even though the server now refuses to replace a live peer.
These tests do not validate real TTY presentation, hardware drivers or hotplug.
