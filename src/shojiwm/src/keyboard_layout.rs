//! Active seat layout for TypeScript shell integration; never records keys.
use crate::state::ShojiWM;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct KeyboardLayoutSnapshot {
    pub index: u32,
    pub name: String,
}

impl ShojiWM {
    pub fn sync_keyboard_layout(&mut self) {
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let layout = keyboard.with_xkb_state(self, |context| {
            let xkb = context.xkb().lock().unwrap();
            let layout = xkb.active_layout();
            KeyboardLayoutSnapshot {
                index: layout.0,
                name: xkb.layout_name(layout).to_owned(),
            }
        });
        if let Some(evaluator) = self.decoration_evaluator.as_embedded()
            && evaluator.set_keyboard_layout(layout)
        {
            // Wake even an idle runtime so panels receive the new layout.
            self.runtime_scheduler_enabled = true;
        }
    }
}
