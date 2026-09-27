---
sidebar_position: 4
---

# Input devices

`COMPOSITOR.input` configures keyboards, pointers (mice), and touchpads. As with
outputs, you register a factory that the compositor calls whenever the set of
input devices changes; you mutate the draft it passes in.

```ts
COMPOSITOR.input.configure((input, context) => {
  // Global defaults applied to all matching devices
  input.global = {
    touchpad: {
      tapToClick: true,
      naturalScroll: true,
      scrollMethod: 'twoFinger',
      disableWhileTyping: true,
      scrollFactor: 0.3,
    },
    pointer: {
      pointerAccel: 0.0,
      accelProfile: 'flat',
    },
    keyboard: {
      options: 'caps:ctrl_modifier',
      repeatRate: 60,
      repeatDelay: 250,
    },
  };

  // Per-device override, keyed by device name
  input.device['Razer Razer Blade Keyboard'] = {
    keyboard: {layout: 'us'},
  };
});
```

`input.global` applies to every device of that kind. `input.device[name]`
overrides settings for one specific device (set it to `null` to clear an
override). Each value is an `InputDeviceConfig` with optional `keyboard`,
`pointer`, and `touchpad` sub-objects.

## Keyboard settings

`keyboard` — an `InputDeviceConfig.keyboard` object. The `rules`/`model`/
`layout`/`variant`/`options` fields are standard XKB settings.

| Field | Type | Meaning |
| --- | --- | --- |
| `layout` | `string` | XKB layout, e.g. `"us"`, `"jp"`, `"de"` |
| `variant` | `string` | Layout variant, e.g. `"dvorak"` |
| `options` | `string` | XKB options, e.g. `"caps:ctrl_modifier"`, `"ctrl:nocaps"` |
| `rules` | `string` | XKB rules set |
| `model` | `string` | Keyboard model |
| `repeatRate` | `number` | Key repeats per second |
| `repeatDelay` | `number` | Delay (ms) before key repeat starts |

```ts
input.global = {
  keyboard: {layout: 'us', options: 'caps:ctrl_modifier', repeatRate: 60, repeatDelay: 250},
};
```

## Reading the active keyboard layout

The compositor publishes the seat's actual XKB layout to
`$XDG_RUNTIME_DIR/shojiwm-$WAYLAND_DISPLAY-keyboard.json` for external panels:

```json
{"pid": 1234, "index": 0, "name": "English (US)"}
```

`index` is the zero-based XKB layout index and `name` is its XKB display name.
The file contains no key events or typed text. It is written on the first
scheduler tick and whenever the layout changes, with owner-only permissions
and an atomic replacement. Watch the parent directory for replacements,
rather than holding the original file open. No file is published without
`XDG_RUNTIME_DIR`.

The file may remain after the compositor exits; readers should stop showing
it when the Wayland session disconnects. `pid` identifies the publisher.
A failed write is logged once and retried on the next layout change.

## Pointer (mouse) settings

`pointer` — an `InputDeviceConfig.pointer` object.

| Field | Type | Meaning |
| --- | --- | --- |
| `pointerAccel` | `number` | Acceleration speed, `-1.0`…`1.0` |
| `accelProfile` | `"adaptive" \| "flat"` | Acceleration curve (`"flat"` = 1:1, no accel) |
| `naturalScroll` | `boolean` | Reverse scroll direction |
| `leftHanded` | `boolean` | Swap left/right buttons |
| `middleEmulation` | `boolean` | Emulate middle click via left+right |

```ts
input.global = {pointer: {accelProfile: 'flat', pointerAccel: 0.0}};
```

## Touchpad settings

`touchpad` — a `TouchpadInputConfig`. It **extends the pointer settings above**,
so every `pointer` field is also valid here, plus:

| Field | Type | Meaning |
| --- | --- | --- |
| `tapToClick` | `boolean` | Tap the pad to click |
| `tapButtonMap` | `"leftRightMiddle" \| "leftMiddleRight"` | Multi-finger tap → button mapping |
| `clickMethod` | `"buttonAreas" \| "clickfinger"` | How physical clicks map to buttons |
| `scrollMethod` | `"none" \| "twoFinger" \| "edge" \| "onButtonDown"` | How scrolling is triggered |
| `scrollFactor` | `number` | Scroll speed multiplier |
| `disableWhileTyping` | `boolean` | Ignore the pad while typing |

…plus all pointer fields (`pointerAccel`, `accelProfile`, `naturalScroll`,
`leftHanded`, `middleEmulation`).

```ts
input.global = {
  touchpad: {
    tapToClick: true,
    naturalScroll: true,
    scrollMethod: 'twoFinger',
    scrollFactor: 0.3,
    disableWhileTyping: true,
  },
};
```

## Reading and targeting devices

The factory's second argument (and the controller itself) exposes the connected
devices, so you can apply settings conditionally by kind.

| Member | Returns |
| --- | --- |
| `devices` | `InputDeviceInfo[]` |
| `current` | `Record<string, InputDeviceInfo>` |
| `get(key)` | `InputDeviceInfo \| undefined` |
| `find(predicate)` | first matching device |
| `configure(factory)` | register a config factory |
| `reconfigure()` | re-run all factories now |

Each `InputDeviceInfo` has `key`, `name`, optional `vendor`/`product`, and a
`kind` flag object (`keyboard`, `pointer`, `touchpad`, `touch`, `tabletTool`,
`tabletPad`, `gesture`, `switch`).

```ts
COMPOSITOR.input.configure((input, ctx) => {
  for (const device of ctx.devices) {
    if (device.kind.touchpad) {
      input.device[device.key] = {touchpad: {scrollMethod: 'twoFinger'}};
    }
  }
});
```
