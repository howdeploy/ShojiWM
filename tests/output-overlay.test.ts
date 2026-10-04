import assert from "node:assert/strict";
import { test } from "node:test";
import {
  COMPOSITOR, noise, compileEffect, compileWindowEffect, compileLayerEffect, compilePopupEffect,
  windowSource, layerSource, popupSource, type CompositorEffectConfig,
} from "../packages/shoji_wm/src/index";
import {
  compileOverlayEffect, installOverlayBridge, overlay, snapshotSource,
  type OverlayBridge,
} from "../packages/shoji_wm/src/overlay";
import { signal } from "../packages/shoji_wm/src/signals";
import { backdropSource, shaderStage, loadShader } from "../packages/shoji_wm/src/shader";

function deferred() {
  let resolve!: () => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<void>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

function native() {
  const ready = deferred();
  const closed = deferred();
  const updates: unknown[] = [];
  let creates = 0;
  const requests: Parameters<OverlayBridge["createOverlay"]>[] = [];
  let disposals = 0;
  const bridge: OverlayBridge = {
    createOverlay(...args) { creates++; requests.push(args); return 1; },
    waitOverlay() { return ready.promise; },
    waitOverlayClosed() { return closed.promise; },
    updateOverlay(_id, value) { updates.push(value); },
    disposeOverlay() { disposals++; closed.resolve(); },
  };
  return { bridge, ready, closed, updates, requests, get creates() { return creates; }, get disposals() { return disposals; } };
}

const options = () => ({ effect: compileOverlayEffect({ input: snapshotSource(), pipeline: [noise({ amount: 0 })] }) });
const nextTurn = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

test("overlay captures before resolving, streams signals, and unsubscribes on close", async () => {
  const host = native();
  installOverlayBridge(host.bridge);
  const progress = signal(0);
  let resolved = false;
  const pending = overlay("DP-1", { effect: compileOverlayEffect({
    input: snapshotSource(), alpha: "preserve",
    pipeline: [shaderStage(loadShader("/test.frag"), { uniforms: { progress } })],
  }) }).then((handle) => { resolved = true; return handle; });
  await nextTurn();
  assert.equal(host.creates, 1);
  assert.deepEqual(host.requests[0].slice(0, 3), ["DP-1", "top", 10_000]);
  assert.equal(host.requests[0][4], false);
  assert.equal(resolved, false);
  progress.value = 0.25;
  const latest = host.updates.at(-1) as { pipeline: [{ uniforms: { progress: number } }] };
  assert.equal(latest.pipeline[0].uniforms.progress, 0.25);
  host.ready.resolve();
  const handle = await pending;
  host.closed.resolve(); // timeout/replacement/reload closes without user disposal
  await handle.closed;
  const count = host.updates.length;
  progress.value = 0.5;
  assert.equal(host.updates.length, count);
  handle.dispose();
  handle.dispose();
  assert.equal(host.disposals, 0);
});

test("output objects, placement, deadline and resolved effects reach the native bridge", async () => {
  const host = native();
  installOverlayBridge(host.bridge);
  host.ready.resolve();
  const effect = options().effect;
  const handle = await overlay({ name: "HDMI-A-1" }, {
    effect, placement: "below-layers", maxDuration: 1200,
  });
  assert.deepEqual(host.requests, [["HDMI-A-1", "below-layers", 1200, effect.effect, false]]);
  handle.dispose();
  await handle.closed;
});

test("persistent live overlays forward their lifetime and stop updates on disposal", async () => {
  const host = native();
  installOverlayBridge(host.bridge);
  host.ready.resolve();
  const pixels = signal(12);
  const effect = compileOverlayEffect({ input: backdropSource(),
    pipeline: [shaderStage(loadShader("/pixelation.frag"), { uniforms: { pixels } })],
  });
  const handle = await overlay("DP-1", { effect, persistent: true });
  assert.deepEqual(host.requests[0].slice(0, 3), ["DP-1", "top", 10_000]);
  assert.equal(host.requests[0][4], true);
  pixels.value = 24;
  assert.equal((host.updates.at(-1) as { pipeline: [{ uniforms: { pixels: number } }] }).pipeline[0].uniforms.pixels, 24);
  handle.dispose();
  await handle.closed;
  const count = host.updates.length;
  pixels.value = 8;
  assert.equal(host.updates.length, count);
});

test("capture failure rejects and explicit disposal is idempotent", async () => {
  const failed = native();
  installOverlayBridge(failed.bridge);
  const pending = overlay("DP-1", options());
  const rejected = assert.rejects(pending, /shader failed/);
  await nextTurn();
  failed.ready.reject(new Error("shader failed"));
  await rejected;
  assert.equal(failed.disposals, 1);

  const host = native();
  installOverlayBridge(host.bridge);
  host.ready.resolve();
  const handle = await overlay("DP-1", options());
  handle.dispose();
  handle.dispose();
  await handle.closed;
  assert.equal(host.disposals, 1);
});

test("awaited compositor callbacks fail before queuing capture; detached tasks proceed", async () => {
  const host = native();
  let handling = true;
  installOverlayBridge(host.bridge, () => !handling);
  await assert.rejects(overlay("DP-1", options()), /detached task/);
  assert.equal(host.creates, 0);
  const pending = overlay("DP-1", options());
  handling = false; // synchronous handler returns to the compositor
  host.ready.resolve();
  const handle = await pending;
  assert.equal(host.creates, 1);
  handle.dispose();
  await handle.closed;
});

test("legacy window, layer and popup plugins survive config assignment and active overlays", async () => {
  const original = COMPOSITOR.effect;
  const host = native();
  installOverlayBridge(host.bridge);
  host.ready.resolve();
  const background = compileEffect({ input: backdropSource(), pipeline: [noise({ amount: 0 })] });
  const window = () => ({ replace: compileWindowEffect({ input: windowSource(), pipeline: [noise({ amount: 0 })] }) });
  const layer = () => ({ inFront: compileLayerEffect({ input: layerSource(), pipeline: [noise({ amount: 0 })] }) });
  const popup = () => ({ replace: compilePopupEffect({ input: popupSource(), pipeline: [noise({ amount: 0 })] }) });
  const oldConfig: CompositorEffectConfig = { background_effect: background, window, layer, popup };
  let handle: Awaited<ReturnType<typeof overlay>> | undefined;
  try {
    COMPOSITOR.effect = oldConfig;
    handle = await COMPOSITOR.effect.overlay("DP-1", { ...options(), persistent: true });
    assert.equal(COMPOSITOR.effect.overlay, overlay);
    assert.equal(COMPOSITOR.effect.background_effect, background);
    assert.equal(COMPOSITOR.effect.window, window);
    assert.equal(COMPOSITOR.effect.layer, layer);
    assert.equal(COMPOSITOR.effect.popup, popup);
    assert.equal("overlay" in oldConfig, false);
    handle.dispose();
    await handle.closed;
    COMPOSITOR.effect = { background_effect: null };
    assert.equal(COMPOSITOR.effect.overlay, overlay);
    assert.equal(COMPOSITOR.effect.layer, undefined);
  } finally { handle?.dispose(); COMPOSITOR.effect = original; }
});

test("invalid safety deadlines do not allocate native overlays", async () => {
  const host = native();
  installOverlayBridge(host.bridge);
  for (const maxDuration of [0, -1, Infinity, NaN, 2_147_483_648]) {
    for (const persistent of [false, true]) {
      await assert.rejects(overlay("DP-1", { ...options(), maxDuration, persistent }), RangeError);
    }
  }
  await assert.rejects(overlay("DP-1", { ...options(), persistent: "true" as unknown as boolean }), TypeError);
  assert.equal(host.creates, 0);
});
