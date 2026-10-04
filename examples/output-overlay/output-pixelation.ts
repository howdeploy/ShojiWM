import {
  COMPOSITOR, backdropSource, compileOverlayEffect, loadShader, shaderStage, signal,
  type OverlayHandle, type Signal,
} from "shoji_wm";

// Copy both pixelation files into your config's src/effect directory.
const shader = loadShader("./src/effect/output-pixelation.frag");
const active = new Map<string, { pixels: Signal<number>; handle?: OverlayHandle }>();

function size(value: number): number {
  if (!Number.isFinite(value)) throw new RangeError("Pixel size must be finite");
  return Math.max(1, Math.min(128, value));
}

export function setPixelSize(output: string, pixels: number): void {
  const value = size(pixels);
  const entry = active.get(output);
  if (entry) entry.pixels.value = value;
}

export async function enablePixelation(output: string, pixels = 12): Promise<void> {
  const value = size(pixels);
  if (active.has(output)) { setPixelSize(output, value); return; }
  const entry: { pixels: Signal<number>; handle?: OverlayHandle } = { pixels: signal(value) };
  active.set(output, entry);
  try {
    const handle = await COMPOSITOR.effect.overlay(output, {
      persistent: true,
      placement: "top",
      effect: compileOverlayEffect({
        input: backdropSource(), alpha: "preserve",
        pipeline: [shaderStage(shader, { uniforms: { pixelSize: entry.pixels } })],
      }),
    });
    // Retire a cancelled request as soon as its first-frame handle is available.
    if (active.get(output) !== entry) { handle.dispose(); return; }
    entry.handle = handle;
    void handle.closed.then(() => { if (active.get(output) === entry) active.delete(output); });
  } catch (error) {
    if (active.get(output) === entry) { active.delete(output); throw error; }
  }
}

export function disablePixelation(output: string): void {
  active.get(output)?.handle?.dispose();
  active.delete(output);
}

export function togglePixelation(output: string): void {
  if (active.has(output)) disablePixelation(output);
  else void enablePixelation(output).catch(console.error);
}
