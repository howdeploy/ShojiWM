# Fork integration branch

`howdeploy/stack` integrates upstream
`54cb56b3349b38cd1541fd42556e03f38d603a7c` with the remaining local changes:

| Branch | Change |
| --- | --- |
| `feat/keyboard-layout-status` | Emit the active XKB layout through `COMPOSITOR.event.onKeyboardLayoutChange` (PR #105) |
| `fix/native-cursor-scale` | Preserve native Wayland cursor sizes and limit the legacy fallback to Xwayland bridge clients |
| `feat/native-workspace-transitions` | Optional native TTY workspace wave API |

Upstream has merged our keyboard LED and full-window subsurface fixes (#106 and
#108). It also supplies an embedded xwayland-satellite and its cursor fixes in
place of PR #107. The integration keeps our native-cursor safeguards and uses
upstream's scoped viewport check, without applying the legacy scale heuristic
to viewport-corrected cursors. `main` tracks the upstream baseline; open PR
branches remain independently reviewable.

## Build and install

Follow the dependencies in the [installation guide](docs/docs/getting-started/installation.md),
then build this branch using the existing upstream installer:

```sh
git clone --branch howdeploy/stack https://github.com/howdeploy/ShojiWM.git
cd ShojiWM
./dist/install.sh
```

The installer builds and installs the compositor and portal. It requires sudo
for system installation. This fork supplies the upstream default configuration;
it does not ship a personal desktop, panel, application configuration, or themes.
The workspace wave is opt-in through the API documented in
[Workspace wave transitions](docs/docs/configuration/workspace-transitions.md).
External panels can receive the layout status through the TypeScript event described in
[Input devices](docs/docs/configuration/input.md).

The earlier draft's automatic `shojiwm-$WAYLAND_DISPLAY-keyboard.json` file is
no longer written by Rust. Before installing this branch with a panel that
reads that file, publish it from the TypeScript event handler or migrate the
panel to your IPC transport. This repository does not update personal configs.

## Validation status

The keyboard-layout PR and the integration branch compile with Rust 1.94.0;
the embedded-runtime keyboard-layout regression test passes in both. It covers
initial delivery, index/name changes, deduplication across
fast and full scheduler requests, unsubscribe, coalescing, and reload.
The integration branch has not been validated in a graphical session. Cursor
rendering, GPU effects, hardware LEDs, and multi-output behavior still require
runtime checks. No installed compositor or personal configuration was changed
as part of this integration.

Only explicit source/documentation file lists were committed. New patches and
commit metadata were inspected for personal paths, email addresses, credentials,
logs, screenshots, and application configuration. No personal configuration,
diagnostic artifacts, or locally compiled binaries are included.
