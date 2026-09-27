//! Read-only shell status from the real seat keymap; never records keys.
use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};

use crate::state::ShojiWM;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct KeyboardLayoutStatus {
    pid: u32,
    index: u32,
    name: String,
}

fn publish(path: &std::path::Path, status: &KeyboardLayoutStatus) -> std::io::Result<()> {
    let temporary = path.with_extension("json.tmp");
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporary)?;
    serde_json::to_writer(&mut file, status)?;
    file.write_all(b"\n")?;
    // Readers see complete snapshots and can watch the atomic replacement.
    std::fs::rename(temporary, path)
}

impl ShojiWM {
    pub fn sync_keyboard_layout(&mut self) {
        let Some(keyboard) = self.seat.get_keyboard() else { return };
        let status = keyboard.with_xkb_state(self, |context| {
            let xkb = context.xkb().lock().unwrap();
            let layout = xkb.active_layout();
            KeyboardLayoutStatus { pid: std::process::id(), index: layout.0, name: xkb.layout_name(layout).to_owned() }
        });
        if self.published_keyboard_layout.as_ref() == Some(&status) { return; }
        let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") else { return };
        let path = PathBuf::from(runtime_dir).join(format!("shojiwm-{}-keyboard.json", self.socket_name.to_string_lossy()));
        if let Err(error) = publish(&path, &status) {
            tracing::warn!(?error, "could not publish keyboard layout status");
        }
        // A failed write is reported once per layout, not on every scheduler tick.
        self.published_keyboard_layout = Some(status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_complete_layout_snapshots() {
        let path = std::env::temp_dir().join(format!("shoji-keyboard-status-test-{}.json", std::process::id()));
        for (index, name) in [(0, "English (US)"), (1, "Russian") ] {
            let status = KeyboardLayoutStatus { pid: std::process::id(), index, name: name.into() };
            publish(&path, &status).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            assert_eq!(value["index"], index);
            assert_eq!(value["name"], name);
            assert_eq!(value.as_object().unwrap().len(), 3);
        }
        std::fs::remove_file(path).unwrap();
    }
}
