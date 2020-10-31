#[allow(clippy::wildcard_imports)]
use pebble_sys::graphics::graphics_types::*;

use crate::Handle;

pub mod color_definitions;

pub type Color8 = GColor8;
pub type Rectangle = GRect;

pub struct Bitmap<'a>(Handle<'a, GBitmap>);

impl Bitmap<'static> {
	/// # Errors
	///
	/// Iff the Bitmap could not be created.
	pub fn from_png_data(png_data: &[u8]) -> Result<Self, ()> {
		let len = png_data.len();
		let gbitmap =
			unsafe { gbitmap_create_from_png_data(png_data.as_ptr_range().start, len) }.ok_or(())?;
		Ok(Self(Handle::new(gbitmap)))
	}
}

impl<'a> Bitmap<'a> {
	#[must_use]
	pub fn bounds(&self) -> Rectangle {
		unsafe { gbitmap_get_bounds(&*self.0) }
	}

	#[must_use]
	pub fn as_sys(&self) -> &GBitmap {
		&*self.0
	}
}

impl<'a> Drop for Bitmap<'a> {
	fn drop(&mut self) {
		//TODO: This will become similar to NumberWindow's Drop implementation, just without the specialisation.
		unsafe { gbitmap_destroy(&mut *(self.0.duplicate().as_mut_unchecked() as *mut _)) }
	}
}
