use super::window::Window;
#[allow(clippy::wildcard_imports)]
use pebble_sys::user_interface::window_stack::*;

pub fn push<T: ?Sized>(window: &Window<T>, animated: bool) {
	unsafe {
		//SAFETY: This wrapper does not allow reacquiring windows from the window stack.
		window_stack_push(&mut *(window.0.as_mut_unchecked() as *mut _), animated)
	}
}

#[allow(clippy::must_use_candidate)] // side effects
pub fn pop(animated: bool) -> bool {
	unsafe { window_stack_pop(animated) }.is_some()
}

pub fn pop_all(animated: bool) {
	unsafe { window_stack_pop_all(animated) }
}

#[allow(clippy::must_use_candidate)] // side effects
pub fn remove<T: ?Sized>(window: &Window<T>, animated: bool) -> bool {
	unsafe { window_stack_remove(window.0.as_mut_unchecked(), animated) }
}

#[must_use]
pub fn is_empty() -> bool {
	unsafe { window_stack_get_top_window() }.is_none()
}

#[must_use]
pub fn contains_window<T: ?Sized>(window: &Window<T>) -> bool {
	unsafe { window_stack_contains_window(window.0.as_mut_unchecked()) }
}
