# KISA desktop compositor snapshot

The fork's `main` branch preserves the current local KISA ShojiWM desktop
source as of 2026-10-27, on top of upstream integration commit `dcea1dd` and
the previous desktop snapshot `dcb73aa`. The installed reference package is
`shojiwm-local 0.1.0.r20260929.dcea1dd.overlays-13`.
`howdeploy/desktop-20261004` remains an archival snapshot; `main` is the entry
point for compatibility with KISA Stack. This is the author's fork,
distinct from upstream `bea4dev/ShojiWM` and the earlier
`howdeploy/stack` branch. Desktop configuration is published in
[KISA Stack](https://github.com/howdeploy/kisa-stack/tree/main/dotfiles).

This snapshot includes generic output shader overlays and their watchdog,
reload/cache cleanup, the local same-user logout socket, and client input-region
fallthrough. The generic input-region fix was accepted upstream in
[PR #122](https://github.com/bea4dev/ShojiWM/pull/122).

The October update also preserves:

- Output power control (DPMS), `COMPOSITOR.output.setPower`, wake-on-input
  and the wlr-output-power-management protocol, adapted to this older runtime.
- Background-client focus protection, foreground-only popup grabs and
  panel-client foreign-toplevel activation.
- Active-window pointer constraints, confinement-edge movement, per-device
  keyboard sources and release of held keys when a keyboard is disconnected.
- The local dock hit-test deadlock fix, without changing the desktop-pet policy.
- Portal thumbnail resource cleanup, bounded source updates, picker
  cancellation and strict restoration by the original window identifier.

The older native `COMPOSITOR.workspace.transition` API remains available for
the existing desktop configuration. This snapshot does not merge current
upstream or migrate to its split compositor/config-runtime architecture.

Two desktop-pet policies remain deliberately local: excluding app ID
`MateEngineX.x86_64` from the final automatic-focus fallback, and letting that
pet's shaped input area take precedence over the `shoji-dock` surface (except
dock popups). These are integration policies, not generic upstream APIs.
MateEngine motion, workspace transfer and sitting are handled by the TypeScript
configuration and the application bridge, not by X11 position requests.

## Source provenance and verification limits

The compositor, SDK and decoration runtime were copied from the current
`shojiwm-output-overlays` working tree. On 2026-10-27, its release binary
matched the installed `/usr/bin/shoji_wm` byte for byte (SHA-256
`96fef219d9681608a7d122fcfa832f4576c365d62b6fabaefbac1a8d6834f69e`).
Its SDK and decoration-runtime files also matched the installed copies.

The portal's three modified source files were copied from the current
`shojiwm-fork` working tree. Its crate manifest matches the snapshot base;
the compositor's existing workspace manifests and lockfile are retained.
The installed portal binary has SHA-256
`b063784690784b9a7a4bf8b0c412b81a92ba16cef70b6ff216161e9d0dd9a96b`;
an exact match to a local portal build artifact was not established.

The earlier local builds were installed on the author's desktop. Publishing
this source snapshot did not run tests, a fresh build or graphical checks.
Binary equality and source copying do not prove a reproducible clean build
or that the currently running session has loaded every installed component.
It ships no personal configuration, application assets, logs or compiled binaries.
Follow the upstream installation instructions to build it; read the pinned
dotfiles documentation before configuring the desktop.
