use pebble_sys::foundation::resources::ResHandle;
#[allow(clippy::wildcard_imports)]
use pebble_sys::foundation::resources::*;

pub type ResourceHandle = ResHandle;

#[must_use]
pub fn get_handle(resource_id: u32) -> ResHandle {
	unsafe {
		//TODO: How does this behave with invalid resource IDs?
		resource_get_handle(resource_id)
	}
}

#[must_use]
pub fn size(resource_handle: ResHandle) -> usize {
	unsafe { resource_size(resource_handle) }
}
