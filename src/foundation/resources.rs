use crate::Box;
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

/// # Errors
///
/// Iff not enough heap memory for the resource could be allocated.
///
/// # Panics
///
/// Iff the heap allocation succeeds but not all data is read. This shouldn't happen.
pub fn load(resource_handle: ResHandle) -> Result<Box<'static, [u8]>, ()> {
	let size = size(resource_handle);
	let mut buffer = Box::new_buffer_uninit(size)?;
	let loaded = unsafe { resource_load(resource_handle, buffer.as_mut_ptr() as *mut _, size) };
	assert_eq!(size, loaded);
	Ok(Box::assume_init(buffer))
}
