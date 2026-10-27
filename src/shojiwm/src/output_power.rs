//! Output power (DPMS): switching a panel off without taking its output out of
//! the layout. Windows stay where they are and the output keeps its wl_output;
//! only scanout stops, which is what an idle daemon wants, unlike
//! `mode: "disabled"`.
//!
//! The power state lives here, keyed by output name. Requests come from
//! `wlr-output-power-management` clients (`wlopm`), from the config
//! (`COMPOSITOR.output.setPower`) and from input when an output was switched
//! off with `wake_on_input`. The TTY backend applies it
//! ([`crate::backend::tty::sync_output_power`]); the nested backend has no
//! panel to switch, so nothing is power managed there.

use smithay::output::Output;
use std::sync::Mutex;
use tracing::{info, warn};

use crate::{protocols::output_power::OutputPowerHandler, state::ShojiWM};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputPowerMode {
    On,
    Off,
    /// Off when any targeted output is on, otherwise on.
    Toggle,
}

impl OutputPowerMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "on" => Some(Self::On),
            "off" => Some(Self::Off),
            "toggle" => Some(Self::Toggle),
            _ => None,
        }
    }

    /// Whether the targeted outputs end up on, given whether any of them is on now.
    pub fn resolve(self, any_on: bool) -> bool {
        match self {
            Self::On => true,
            Self::Off => false,
            Self::Toggle => !any_on,
        }
    }
}

/// A power change requested by the config runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOutputPowerRequest {
    pub mode: OutputPowerMode,
    /// Output name; `None` targets every output.
    pub output: Option<String>,
    /// When switching off: switch back on at the next input event.
    pub wake_on_input: bool,
}

// This runtime uses SIGUSR1 rather than upstream's newer host-message channel.
static RUNTIME_REQUESTS: Mutex<Vec<RuntimeOutputPowerRequest>> = Mutex::new(Vec::new());

pub(crate) fn queue_runtime_request(request: RuntimeOutputPowerRequest) {
    RUNTIME_REQUESTS.lock().unwrap().push(request);
    crate::ssd::wake_compositor();
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoweredOffOutput {
    pub wake_on_input: bool,
}

impl ShojiWM {
    /// Redraw each output once after wake-up; the debug flag is persistent.
    pub fn request_full_damage(&mut self) {
        let names: Vec<_> = self.space.outputs().map(Output::name).collect();
        self.full_damage_pending_outputs.extend(names);
        self.schedule_redraw();
    }

    pub fn take_full_damage(&mut self, output_name: &str) -> bool {
        self.full_damage_pending_outputs.remove(output_name) || self.force_full_damage
    }

    pub(crate) fn apply_pending_output_power_requests(&mut self) {
        let requests = std::mem::take(&mut *RUNTIME_REQUESTS.lock().unwrap());
        for request in requests {
            self.apply_runtime_output_power_request(request);
        }
    }

    pub fn output_powered_off(&self, output_name: &str) -> bool {
        self.powered_off_outputs.contains_key(output_name)
    }

    /// Whether screen capture of `output_name` can produce a frame: it must be
    /// rendered, which a disabled or powered-off output is not.
    pub fn output_capturable(&self, output_name: &str) -> bool {
        self.runtime_output_render_enabled(output_name) && !self.output_powered_off(output_name)
    }

    /// Outputs with a panel to switch: those the TTY backend drives and the
    /// display configuration does not disable.
    fn power_manageable_outputs(&self) -> Vec<Output> {
        crate::backend::tty::tty_connected_outputs(self)
            .into_iter()
            .filter(|output| self.runtime_output_render_enabled(&output.name()))
            .collect()
    }

    /// Switch `output` on or off. Returns `false` when it cannot be power
    /// managed. `wake_on_input` only matters when switching off.
    pub fn set_output_power_state(
        &mut self,
        output: &Output,
        on: bool,
        wake_on_input: bool,
    ) -> bool {
        if !self.power_manageable_outputs().contains(output) {
            return false;
        }
        let name = output.name();
        let was_on = !self.output_powered_off(&name);
        if on {
            self.powered_off_outputs.remove(&name);
        } else {
            self.powered_off_outputs
                .insert(name.clone(), PoweredOffOutput { wake_on_input });
        }
        if was_on != on {
            info!(output = %name, on, wake_on_input, "output power changed");
            if !on {
                // Nothing renders while off, so queued captures would wait until power-on.
                // Capture clients (the screencast portal) back off and retry.
                self.screencopy_state.remove_output(output);
                crate::backend::image_copy_capture_render::fail_pending_output_capture(
                    &mut self.image_copy_capture_pending,
                    output,
                    smithay::wayland::image_copy_capture::CaptureFailureReason::Unknown,
                );
            }
            crate::backend::tty::sync_output_power(self);
            self.output_power_manager_state.mode_changed(output, on);
        }
        true
    }

    pub fn apply_runtime_output_power_request(&mut self, request: RuntimeOutputPowerRequest) {
        let outputs = self.power_manageable_outputs();
        let targets: Vec<Output> = match &request.output {
            Some(name) => outputs
                .into_iter()
                .filter(|output| &output.name() == name)
                .collect(),
            None => outputs,
        };
        if targets.is_empty() {
            warn!(
                output = request.output.as_deref().unwrap_or("*"),
                "output power request matched no output that can be power managed"
            );
            return;
        }
        let any_on = targets
            .iter()
            .any(|output| !self.output_powered_off(&output.name()));
        if request.mode == OutputPowerMode::Toggle && self.input_woke_outputs {
            // The key press of this very binding already woke the outputs: that was the toggle.
            return;
        }
        let on = request.mode.resolve(any_on);
        for output in &targets {
            self.set_output_power_state(output, on, request.wake_on_input);
        }
    }

    /// Input arrived: switch on the outputs that were switched off with
    /// `wake_on_input`.
    pub fn wake_outputs_on_input(&mut self) {
        if !self
            .powered_off_outputs
            .values()
            .any(|off| off.wake_on_input)
        {
            return;
        }
        self.power_on_outputs(|off| off.wake_on_input);
        self.input_woke_outputs = true;
    }

    /// Switch every output back on, e.g. when the user returns to this VT:
    /// whoever powered them off (an idle daemon) may be gone by then.
    pub fn power_on_all_outputs(&mut self) {
        if !self.powered_off_outputs.is_empty() {
            self.power_on_outputs(|_| true);
        }
    }

    fn power_on_outputs(&mut self, select: impl Fn(&PoweredOffOutput) -> bool) {
        let outputs: Vec<Output> = self
            .power_manageable_outputs()
            .into_iter()
            .filter(|output| {
                self.powered_off_outputs
                    .get(&output.name())
                    .is_some_and(&select)
            })
            .collect();
        for output in &outputs {
            self.set_output_power_state(output, true, false);
        }
    }

    /// `output` left the layout (unplugged or disabled). Forget its power
    /// state so it comes back on, and fail its protocol controls.
    pub(crate) fn forget_output_power(&mut self, output: &Output) {
        self.full_damage_pending_outputs.remove(&output.name());
        if self.powered_off_outputs.remove(&output.name()).is_some() {
            crate::backend::tty::sync_output_power(self);
        }
        self.output_power_manager_state.output_removed(output);
    }
}

impl OutputPowerHandler for ShojiWM {
    fn output_power_manager_state(
        &mut self,
    ) -> &mut crate::protocols::output_power::OutputPowerManagerState {
        &mut self.output_power_manager_state
    }

    fn output_power(&self, output: &Output) -> Option<bool> {
        self.power_manageable_outputs()
            .contains(output)
            .then(|| !self.output_powered_off(&output.name()))
    }

    fn set_output_power(&mut self, output: &Output, on: bool) -> bool {
        self.set_output_power_state(output, on, false)
    }
}

#[cfg(test)]
mod tests {
    use super::OutputPowerMode;

    #[test]
    fn parses_the_config_spelling() {
        assert_eq!(OutputPowerMode::parse("on"), Some(OutputPowerMode::On));
        assert_eq!(OutputPowerMode::parse("off"), Some(OutputPowerMode::Off));
        assert_eq!(
            OutputPowerMode::parse("toggle"),
            Some(OutputPowerMode::Toggle)
        );
        assert_eq!(OutputPowerMode::parse("Off"), None);
        assert_eq!(OutputPowerMode::parse(""), None);
    }

    /// Toggling a mixed set switches everything off: one key press must leave
    /// the panels in a single state, and "off" is the one the user asked for
    /// while any panel was still lit.
    #[test]
    fn toggle_switches_off_while_any_output_is_on() {
        assert!(!OutputPowerMode::Toggle.resolve(true));
        assert!(OutputPowerMode::Toggle.resolve(false));
        assert!(OutputPowerMode::On.resolve(true));
        assert!(OutputPowerMode::On.resolve(false));
        assert!(!OutputPowerMode::Off.resolve(true));
        assert!(!OutputPowerMode::Off.resolve(false));
    }
}
