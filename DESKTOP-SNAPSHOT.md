# KISA desktop compositor snapshot

Branch `howdeploy/desktop-20261004` preserves the compositor source used by the
KISA ShojiWM desktop on 2026-10-04, on top of upstream integration commit
`dcea1dd`. It is intentionally separate from upstream `main` and the earlier
`howdeploy/stack` branch. Desktop configuration is published in
[KISA Stack](https://github.com/howdeploy/kisa-stack/tree/main/dotfiles).

This snapshot includes generic output shader overlays and their watchdog,
reload/cache cleanup, the local same-user logout socket, and client input-region
fallthrough. The generic input-region fix is separately proposed upstream in
[PR #122](https://github.com/bea4dev/ShojiWM/pull/122), adapted to current `main`.

Two desktop-pet policies remain deliberately local: excluding app ID
`MateEngineX.x86_64` from the final automatic-focus fallback, and letting that
pet's shaped input area take precedence over the `shoji-dock` surface (except
dock popups). These are integration policies, not generic upstream APIs.
MateEngine motion, workspace transfer and sitting are handled by the TypeScript
configuration and the application bridge, not by X11 position requests.

The earlier local builds were installed and used on the author's desktop.
Publishing this source snapshot did not run a fresh build or graphical tests.
It ships no personal configuration, application assets, logs or compiled binaries.
Follow the upstream installation instructions to build it; read the pinned
dotfiles documentation before configuring the desktop.
