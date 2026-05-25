//! Client-side IME via the `zwp_text_input_v3` protocol.
//!
//! smithay-client-toolkit does not wrap text-input, so the manager/text-input
//! objects are driven directly here. The protocol is double-buffered: preedit,
//! commit, and delete events accumulate and are applied together on `done`.
//!
//! The render thread publishes the desired [`input_method::InputMethod`] to
//! [`Shared`]; [`State::sync_ime`] reconciles it against the text input (which
//! lives on this thread). Preedit/commit notifications flow back to the UI as
//! [`Event::InputMethod`] over the command channel.
//!
//! [`Shared`]: crate::handle::Shared

use iced_runtime::core::{Event, input_method};
use smithay_client_toolkit::globals::GlobalData;
use smithay_client_toolkit::reexports::client::protocol::wl_seat::WlSeat;
use smithay_client_toolkit::reexports::client::{Connection, Dispatch, QueueHandle};
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::{
    self,
    ZwpTextInputManagerV3,
};
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_v3::{
    self,
    ContentHint,
    ContentPurpose,
    ZwpTextInputV3,
};

use super::state::State;
use crate::overlay::Command;

/// Per-object user data for a text input (the state lives on [`State`]).
pub(super) struct TextInputData;

/// Map an Iced IME [`input_method::Purpose`] to a text-input content purpose.
fn content_purpose(purpose: input_method::Purpose) -> ContentPurpose {
    match purpose {
        input_method::Purpose::Normal => ContentPurpose::Normal,
        input_method::Purpose::Secure => ContentPurpose::Password,
        input_method::Purpose::Terminal => ContentPurpose::Terminal,
    }
}

impl State {
    /// Create the per-seat text input, if the compositor offers the manager.
    pub(super) fn ensure_text_input(&mut self, seat: &WlSeat, qh: &QueueHandle<Self>) {
        if self.text_input.is_some() {
            return;
        }
        if let Some(manager) = &self.text_input_manager {
            self.text_input = Some(manager.get_text_input(seat, qh, TextInputData));
        }
    }

    /// Reconcile the text input with the IME state the overlay requested.
    ///
    /// Called once per event-loop turn. Enabling requires the text input to have
    /// entered our surface first; geometry is re-sent (and committed) only when
    /// it changes, to avoid flooding the compositor.
    pub(super) fn sync_ime(&mut self) {
        let Some(text_input) = self.text_input.clone() else {
            return;
        };
        let desired = self.shared.ime.lock().expect("ime mutex poisoned").clone();

        match &desired {
            input_method::InputMethod::Enabled {
                cursor, purpose, ..
            } => {
                if !self.ime_entered {
                    return;
                }

                let just_enabled = !self.ime_enabled;
                if just_enabled {
                    text_input.enable();
                    self.ime_enabled = true;
                    self.send_input_method(input_method::Event::Opened);
                } else if desired == self.ime_applied {
                    return;
                }

                text_input
                    .set_content_type(ContentHint::None, content_purpose(*purpose));
                text_input.set_cursor_rectangle(
                    cursor.x as i32,
                    cursor.y as i32,
                    cursor.width.max(1.0) as i32,
                    cursor.height.max(1.0) as i32,
                );
                text_input.commit();
                self.ime_serial = self.ime_serial.wrapping_add(1);
                self.ime_applied = desired;
            },
            input_method::InputMethod::Disabled => {
                if self.ime_enabled {
                    text_input.disable();
                    text_input.commit();
                    self.ime_serial = self.ime_serial.wrapping_add(1);
                    self.ime_enabled = false;
                    self.send_input_method(input_method::Event::Closed);
                }
                self.ime_applied = input_method::InputMethod::Disabled;
            },
        }
    }

    /// Apply the accumulated preedit/commit on a `done` event, emitting the
    /// matching Iced input-method events.
    fn flush_ime(&mut self) {
        let preedit = self.ime_preedit.take();
        let commit = self.ime_commit.take();
        if preedit.is_none() && commit.is_none() {
            return;
        }

        // A commit replaces any preedit, so Iced expects an empty preedit first.
        if let Some(text) = commit {
            self.send_input_method(input_method::Event::Preedit(String::new(), None));
            self.send_input_method(input_method::Event::Commit(text));
        }

        // Apply the new preedit (empty if none was sent this round). Cursor byte
        // offsets of -1 mean the cursor is hidden.
        let (content, selection) = match preedit {
            Some((text, begin, end)) => {
                let selection =
                    (begin >= 0 && end >= 0).then_some(begin as usize..end as usize);
                (text, selection)
            },
            None => (String::new(), None),
        };
        self.send_input_method(input_method::Event::Preedit(content, selection));
    }

    /// Feed an input-method event to the overlay UI.
    fn send_input_method(&self, event: input_method::Event) {
        let _ = self
            .commands
            .send(Command::Event(Event::InputMethod(event)));
    }
}

impl Dispatch<ZwpTextInputManagerV3, GlobalData> for State {
    fn event(
        _state: &mut Self,
        _manager: &ZwpTextInputManagerV3,
        _event: zwp_text_input_manager_v3::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The manager has no events.
    }
}

impl Dispatch<ZwpTextInputV3, TextInputData> for State {
    fn event(
        state: &mut Self,
        _text_input: &ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _data: &TextInputData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_text_input_v3::Event::Enter { surface }
                if surface == state.main_surface || surface == state.overlay_surface =>
            {
                // The protocol resets enable state on enter; re-enable on the
                // next sync if the UI still wants IME.
                state.ime_entered = true;
                state.ime_enabled = false;
                state.ime_applied = input_method::InputMethod::Disabled;
            },
            zwp_text_input_v3::Event::Leave { .. } => {
                state.ime_entered = false;
                if state.ime_enabled {
                    state.ime_enabled = false;
                    state.send_input_method(input_method::Event::Closed);
                }
                state.ime_applied = input_method::InputMethod::Disabled;
                state.ime_preedit = None;
                state.ime_commit = None;
            },
            zwp_text_input_v3::Event::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => {
                state.ime_preedit =
                    Some((text.unwrap_or_default(), cursor_begin, cursor_end));
            },
            zwp_text_input_v3::Event::CommitString { text } => {
                state.ime_commit = Some(text.unwrap_or_default());
            },
            zwp_text_input_v3::Event::DeleteSurroundingText { .. } => {
                // We do not advertise surrounding text, so we cannot honor a
                // deletion request; ignore it.
            },
            zwp_text_input_v3::Event::Done { .. } => state.flush_ime(),
            _ => {},
        }
    }
}
