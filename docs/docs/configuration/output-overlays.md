---
sidebar_position: 11
---

# Output overlays

`COMPOSITOR.effect.overlay(output, options)` draws a fullscreen effect through the
existing ShojiWM shader pipeline on TTY and Winit. It creates no Wayland surface,
input region or keyboard focus. Existing window, layer, popup and background
effects keep their APIs and `shader_main(EffectContext)` contract.

```ts
const handle = await COMPOSITOR.effect.overlay("DP-1", {
  effect: compileOverlayEffect({
    input: snapshotSource(),
    alpha: "preserve",
    pipeline: [shaderStage(loadShader("./src/effect/output-dissolve.frag"), {
      uniforms: { progress }, // a signal owned by this transition
      // textures: { live: backdropSource() },
    })],
  }),
  placement: "top",
  maxDuration: 10_000,
});
// The old frame is now captured and the shader has successfully run.
changeScene();
// Drive progress, then release the effect (use try/finally in real code).
handle.dispose();
```

The complete, self-contained dissolve example is in
`examples/output-overlay/`. Copy its two files into your config's `src/effect/`
directory, then call `void transition(outputName, changeScene).catch(console.error)`
from a synchronous key/event callback. The generic `animate()` in the upstream
proposal is pseudocode; this example uses signals and native timers available today.

**Do not await an overlay at module scope or return its Promise from a callback
the compositor awaits**, such as `onPointerMoveAsync` or `onGestureSwipeAsync`.
The main thread must return to rendering to make the next frame. Overlay creation
yields one timer turn to let detached callbacks finish, then rejects if the runtime
is still handling a compositor request. Module initialization also rejects calls.
The native deadline runs independently of rendering as an additional safeguard.
Launch the transition as a detached task as shown above.

## Sources and placement

| Option | Behavior |
| --- | --- |
| `snapshotSource()` | Frozen capture of the scene at this placement, including the previous overlay in the same slot; never includes the cursor. Captured once per request. |
| `backdropSource()` | Live scene behind this slot. Captured only when referenced and the pipeline needs evaluation; excludes this slot's old output to avoid feedback. |
| `shaderInput()`, `imageSource()`, state textures | Existing generator and multipass inputs; no screen capture unless the pipeline also references a screen source. |
| `top` (default) | Above desktop windows and layer-shell surfaces, below the cursor. Compositor diagnostics may remain in front. |
| `below-layers` | Above windows, below Top/Overlay layer-shell surfaces and layer popups. Bottom/Background layers remain behind windows. |

`below-layers` does **not** mean below windows. A below-window slot is not
included. The existing native `workspace.transition`
wave API remains available. If both APIs run together, a top overlay includes that
wave in its snapshot; a below-layers overlay remains behind it.

Use `alpha: "preserve"` for fades and transparent procedural output. Pixels must
use premultiplied alpha, as with existing layer effects. `capturePadding` must be
zero; window/layer/popup/xray inputs have no subject in an output effect and are
rejected. A pipeline may use `snapshotSource()` in named shader textures as well
as its primary input. Adding or removing snapshot usage requires a new overlay.
As with existing compiled effects, use at least one pipeline stage unless the
primary input is a generator shader; an empty snapshot pipeline is rejected by
the native descriptor decoder.

## Persistent live effects

Set `persistent: true` to keep an overlay after its first successful frame:

```ts
const pixelSize = signal(12); // physical texture pixels
const handle = await COMPOSITOR.effect.overlay("DP-1", {
  persistent: true,
  placement: "top", // includes windows, desktop widgets and layer-shell UI
  effect: compileOverlayEffect({
    input: backdropSource(),
    alpha: "preserve",
    pipeline: [shaderStage(loadShader("./src/effect/output-pixelation.frag"), {
      uniforms: { pixelSize },
    })],
  }),
});
pixelSize.value = 24;
// No lifetime deadline remains after the first frame; explicitly disable it.
handle.dispose();
await handle.closed;
```

Creation has the same detached-task restriction as temporary overlays. In this
mode `maxDuration` (default 10 seconds) limits only acquisition of the first
successful frame. It remains finite and positive. That deadline is removed once
ready; the effect is neither periodically recreated nor given a large timeout.
Omit `persistent`, or set it to `false`, for the original bounded lifetime.

Each output and placement has its own slot. To affect several monitors, create
one handle for each output. `top` includes layer-shell widgets and UI, while the
cursor stays above the effect; `below-layers` leaves Top/Overlay UI untouched.
Neither placement creates input surfaces or intercepts pointer/keyboard events.
The live capture is taken from the freshly composed scene before inserting this
slot's result, including on replacement, so it never reads its own previous frame.
A lower overlay's result may deliberately feed the top slot.

The complete `output-pixelation.ts` / `output-pixelation.frag` example in
`examples/output-overlay/` exports per-output enable, disable, toggle and pixel
size controls. Copy those two files into `src/effect/`, import the functions in
your config and use synchronous callbacks, for example:

```ts
COMPOSITOR.key.bind("pixelation", "Super+P", () => togglePixelation("DP-1"));
COMPOSITOR.key.bind("pixelation-coarse", "Super+Alt+P", () => setPixelSize("DP-1", 24));
// In an event callback, enable both outputs independently:
void enablePixelation("DP-1", 12).catch(console.error);
void enablePixelation("HDMI-A-1", 8).catch(console.error);
// Later: disablePixelation("DP-1");
```

The example uses the default source-damage invalidation: idle scenes reuse the
cached result, and live content or uniform changes trigger reevaluation. There is
no example animation timer. Lens and refraction shaders use the same mode and
`backdropSource()`; change the shader and its uniforms, not compositor code.
For animation independent of scene damage, opt into `invalidate: {kind: "always"}`
and provide a time signal as with other effects.

## Updates, interruption and cleanup

Signals in uniforms and the effect descriptor update an active overlay without
re-evaluating workspace configuration. Multiple changes are coalesced into the
latest descriptor; GPU programs and state textures use existing pipeline caches.
Use `invalidate: {kind: "always"}` for pipelines that need a frame continuously,
for example temporal state textures. This does not inject a time uniform: supply
time explicitly through a signal. The default reuses the result until its inputs
or signals change, including live-source opacity and transform changes.

There is one active overlay per output and placement. A successful replacement
releases the old instance after the new shader succeeds. If the new effect uses
`snapshotSource()`, it captures the old visual result first; a live backdrop
excludes the old instance in that same slot. A capture or
shader failure rejects the new Promise, leaves the previous overlay intact, and
does not insert a failed result. An error after creation closes that instance.

`dispose()` is idempotent. `closed` resolves on disposal, replacement, deadline,
output removal, mode/scale/transform changes, session pause/lock or config reload.
`maxDuration` defaults to 10 seconds, must be finite and positive, and includes
time waiting for the first frame. For persistent overlays it ends at readiness;
all other cleanup conditions still apply, and unlocking does not recreate the
effect automatically. Every exit releases snapshot and pipeline state;
no overlay capture or animation timer runs while there are no overlay requests.

For a cancelled gesture, animate progress back to zero and dispose. Restoring the
workspace or other scene state is the caller's responsibility.

## Validation status

The persistent extension passed its compact checks on 2026-10-02: seven SDK
tests, three native lifetime/cleanup tests, the stable layer-cache-key test and
the offscreen GPU/SDK bridge test with the real pixelation shader. The type
fixture and both examples pass TypeScript 5.9.3. Native tests used Rust 1.94.0,
offline Cargo, isolated IPC and software surfaceless EGL. An asset-root error in
the temporary GPU-test config was corrected before its successful rerun.
A locked, offline release build passed on 2026-10-02 with Rust 1.94.0.
Installation, reload and session/visual checks have not been performed.
The broader results below predate the persistent extension.

Verified on 2026-09-29, using Rust 1.94.0 and TypeScript 6.0 with
`--ignoreDeprecations 6.0` for the repository's existing `baseUrl` setting:

- The compositor builds in release mode. Installation does not activate a new
  compositor binary in an already-running session; log out and back in to use it.
- The 233 default Rust tests run through an isolated IPC namespace. Socket tests
  require permission to create Unix sockets; do not rerun with desktop IPC paths.
- Both explicitly enabled surfaceless EGL tests pass with software rendering.
  The overlay probe checks capture, interruption at half progress, preservation of
  the previous effect on shader failure, live opacity changes and below-layers
  placement at scale 1.5. It also compares a complete nonuniform image and the
  final composed scene, and exercises successful SDK/native capture and disposal.
- Six SDK tests cover output/placement/deadline forwarding, capture ordering, reactive updates, disposal/unsubscription,
  rejected capture, callback deadlock prevention, deadline validation and legacy
  assignment of the complete effect config.
- SDK, default config, example, legacy type fixture and the user's active config
  pass TypeScript checking against this SDK.

The actual Deno/native bridge is also tested for rejected initialization-time calls,
rejected awaited async callbacks and detached capture timeout without a renderer.
Reproduction commands are in `tests/README.md`.

Real TTY presentation, hardware-driver timing, monitor rotation/hotplug and the
visual appearance of existing user shaders still require a session test.

Proposal: [bea4dev's comment on PR #109](https://github.com/bea4dev/ShojiWM/pull/109#issuecomment-5889104745).
