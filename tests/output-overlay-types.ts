import {
  COMPOSITOR, type CompositorEffectConfig, compileWindowEffect, compileLayerEffect, compilePopupEffect,
  compileOverlayEffect, windowSource, layerSource, popupSource, backdropSource, noise,
} from "../packages/shoji_wm/src/index";
import { transition } from "../examples/output-overlay/output-transition";

const oldConfig: CompositorEffectConfig = {
  background_effect: null,
  window: () => ({ replace: compileWindowEffect({ input: windowSource(), pipeline: [] }) }),
  layer: () => ({ inFront: compileLayerEffect({ input: layerSource(), pipeline: [noise({ amount: 0 })] }) }),
  popup: () => ({ replace: compilePopupEffect({ input: popupSource(), pipeline: [noise({ amount: 0 })] }) }),
};
COMPOSITOR.effect = oldConfig;
const create: typeof import("../packages/shoji_wm/src/overlay").overlay = COMPOSITOR.effect.overlay;
void create;
void transition;

// Only typechecked: never import this fixture into a compositor session.
function persistentExample(): ReturnType<typeof create> {
  return create("DP-1", { persistent: true, effect: compileOverlayEffect({
    input: backdropSource(), pipeline: [noise({ amount: 0 })],
  }) });
}
void persistentExample;
