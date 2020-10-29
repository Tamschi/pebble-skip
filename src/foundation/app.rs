#[allow(clippy::wildcard_imports)]
use pebble_sys::foundation::app::*;

pub fn event_loop() {
	unsafe { app_event_loop() }
}
