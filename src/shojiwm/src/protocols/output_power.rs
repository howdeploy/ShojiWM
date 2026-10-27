//! `zwlr_output_power_manager_v1` (wlr-output-power-management-unstable-v1).
//!
//! Lets idle daemons and shells switch a panel off and on (DPMS) without
//! touching the layout, e.g. `wlopm --off '*'` from a swayidle timeout. The
//! compositor owns the power state ([`OutputPowerHandler`]); this module only
//! keeps the control objects so that every change, whoever made it (a client,
//! the config, input waking the output), reaches all of them as a `mode`
//! event.
//!
//! As in wlroots, a control is exclusive per output: `get_output_power` for an
//! output that already has a live control gets `failed`, and so does an output
//! that cannot be power managed (nested backend).

use smithay::output::Output;
use smithay::reexports::wayland_protocols_wlr::output_power_management::v1::server::{
    zwlr_output_power_manager_v1::{self, ZwlrOutputPowerManagerV1},
    zwlr_output_power_v1::{self, Mode, ZwlrOutputPowerV1},
};
use smithay::reexports::wayland_server::backend::ClientId;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource, WEnum,
};

const VERSION: u32 = 1;

pub trait OutputPowerHandler {
    fn output_power_manager_state(&mut self) -> &mut OutputPowerManagerState;
    /// Whether `output` is powered on, or `None` when it cannot be power
    /// managed.
    fn output_power(&self, output: &Output) -> Option<bool>;
    /// Switch `output` on or off; `false` when that is not possible. The
    /// implementation reports an actual change through
    /// [`OutputPowerManagerState::mode_changed`].
    fn set_output_power(&mut self, output: &Output, on: bool) -> bool;
}

#[derive(Debug, Default)]
pub struct OutputPowerManagerState {
    controls: Vec<(ZwlrOutputPowerV1, Output)>,
}

/// The output a control manages; `None` once it failed and became inert.
#[derive(Debug)]
pub struct OutputPowerData {
    output: Option<Output>,
}

fn wire_mode(on: bool) -> Mode {
    if on { Mode::On } else { Mode::Off }
}

impl OutputPowerManagerState {
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: GlobalDispatch<ZwlrOutputPowerManagerV1, ()>,
        D: Dispatch<ZwlrOutputPowerManagerV1, ()>,
        D: Dispatch<ZwlrOutputPowerV1, OutputPowerData>,
        D: 'static,
    {
        display.create_global::<D, ZwlrOutputPowerManagerV1, _>(VERSION, ());
        Self::default()
    }

    /// Tell every control of `output` about its new power mode.
    pub fn mode_changed(&self, output: &Output, on: bool) {
        for (control, _) in self.controls.iter().filter(|(_, owner)| owner == output) {
            control.mode(wire_mode(on));
        }
    }

    /// `output` is gone or can no longer be power managed: fail its controls.
    pub fn output_removed(&mut self, output: &Output) {
        self.controls.retain(|(control, owner)| {
            if owner != output {
                return true;
            }
            control.failed();
            false
        });
    }

    fn remove(&mut self, control: &ZwlrOutputPowerV1) {
        self.controls.retain(|(existing, _)| existing != control);
    }
}

impl<D> GlobalDispatch<ZwlrOutputPowerManagerV1, (), D> for OutputPowerManagerState
where
    D: GlobalDispatch<ZwlrOutputPowerManagerV1, ()>,
    D: Dispatch<ZwlrOutputPowerManagerV1, ()>,
    D: Dispatch<ZwlrOutputPowerV1, OutputPowerData>,
    D: OutputPowerHandler,
    D: 'static,
{
    fn bind(
        _state: &mut D,
        _dh: &DisplayHandle,
        _client: &Client,
        manager: New<ZwlrOutputPowerManagerV1>,
        _data: &(),
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(manager, ());
    }
}

impl<D> Dispatch<ZwlrOutputPowerManagerV1, (), D> for OutputPowerManagerState
where
    D: Dispatch<ZwlrOutputPowerManagerV1, ()>,
    D: Dispatch<ZwlrOutputPowerV1, OutputPowerData>,
    D: OutputPowerHandler,
    D: 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        _manager: &ZwlrOutputPowerManagerV1,
        request: zwlr_output_power_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_power_manager_v1::Request::GetOutputPower { id, output } => {
                let output = Output::from_resource(&output);
                let mode = output
                    .as_ref()
                    .and_then(|output| state.output_power(output));
                let taken = output.as_ref().is_some_and(|output| {
                    state
                        .output_power_manager_state()
                        .controls
                        .iter()
                        .any(|(_, owner)| owner == output)
                });
                match (output, mode) {
                    (Some(output), Some(on)) if !taken => {
                        let control = data_init.init(
                            id,
                            OutputPowerData {
                                output: Some(output.clone()),
                            },
                        );
                        control.mode(wire_mode(on));
                        state
                            .output_power_manager_state()
                            .controls
                            .push((control, output));
                    }
                    _ => {
                        let control = data_init.init(id, OutputPowerData { output: None });
                        control.failed();
                    }
                }
            }
            zwlr_output_power_manager_v1::Request::Destroy => {}
            _ => {}
        }
    }
}

impl<D> Dispatch<ZwlrOutputPowerV1, OutputPowerData, D> for OutputPowerManagerState
where
    D: Dispatch<ZwlrOutputPowerV1, OutputPowerData>,
    D: OutputPowerHandler,
    D: 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        control: &ZwlrOutputPowerV1,
        request: zwlr_output_power_v1::Request,
        data: &OutputPowerData,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_output_power_v1::Request::SetMode { mode } => {
                let on = match mode {
                    WEnum::Value(Mode::On) => true,
                    WEnum::Value(Mode::Off) => false,
                    _ => {
                        control.post_error(
                            zwlr_output_power_v1::Error::InvalidMode,
                            "unknown power mode",
                        );
                        return;
                    }
                };
                // A failed control is inert until the client destroys it.
                let Some(output) = data.output.as_ref() else {
                    return;
                };
                if !state
                    .output_power_manager_state()
                    .controls
                    .iter()
                    .any(|(existing, _)| existing == control)
                {
                    return;
                }
                let before = state.output_power(output);
                if !state.set_output_power(output, on) {
                    state.output_power_manager_state().remove(control);
                    control.failed();
                } else if before == Some(on) {
                    // No change, so no broadcast; still answer the request.
                    control.mode(wire_mode(on));
                }
            }
            zwlr_output_power_v1::Request::Destroy => {}
            _ => {}
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        control: &ZwlrOutputPowerV1,
        _data: &OutputPowerData,
    ) {
        state.output_power_manager_state().remove(control);
    }
}

/// Delegate the `zwlr_output_power_manager_v1` global to
/// [`OutputPowerManagerState`].
#[macro_export]
macro_rules! delegate_output_power {
    ($ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($ty: [
            smithay::reexports::wayland_protocols_wlr::output_power_management::v1::server::zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1: ()
        ] => $crate::protocols::output_power::OutputPowerManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_protocols_wlr::output_power_management::v1::server::zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1: ()
        ] => $crate::protocols::output_power::OutputPowerManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_protocols_wlr::output_power_management::v1::server::zwlr_output_power_v1::ZwlrOutputPowerV1: $crate::protocols::output_power::OutputPowerData
        ] => $crate::protocols::output_power::OutputPowerManagerState);
    };
}
