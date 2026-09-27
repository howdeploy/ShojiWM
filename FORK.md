# Fork integration branch

`howdeploy/stack` combines five independent changes based on upstream
`3fef0c862caab18332947f5e4ede83ff446312a7`:

| Branch | Change |
| --- | --- |
| `feat/keyboard-layout-status` | Publish the active XKB layout for external panels |
| `fix/keyboard-led-state` | Synchronize hardware lock LEDs, including newly attached keyboards |
| `fix/native-cursor-scale` | Restrict the oversized-cursor workaround to Xwayland bridge clients |
| `fix/full-window-effect-subsurfaces` | Include subsurfaces in full-source TTY window effects |
| `feat/native-workspace-transitions` | Optional native TTY workspace wave API |

Each contribution branch starts directly from upstream and is independently
reviewable. `main` remains the upstream baseline.

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
External panels can read the layout status described in
[Input devices](docs/docs/configuration/input.md).

## Validation status

These changes were extracted from a local working compositor stack. The
combined implementation was compared with that source tree: only documentation,
comments, and the missing public TypeScript type export were added during
extraction. The individual branches and this integration branch have not been
rebuilt or runtime-tested after extraction. They are published for review,
not as a verified binary release. Existing regression tests for layout status,
cursor scaling, and the embedded workspace bridge are included in the relevant
branches. GPU effects, hardware LEDs, and multi-output behavior still require
testing of the extracted branches.

Only explicit source/documentation file lists were committed. New patches and
commit metadata were inspected for personal paths, email addresses, credentials,
logs, screenshots, and application configuration. No personal configuration,
diagnostic artifacts, or locally compiled binaries are included.
