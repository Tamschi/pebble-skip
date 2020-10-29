use super::window_stack;
use crate::{graphics::graphics_types::Color8, Box, Handle, SpecialDrop};
use core::{
	marker::PhantomData,
	mem::ManuallyDrop,
	ops::{Deref, DerefMut},
};
use debugless_unwrap::DebuglessUnwrapNone as _;
#[allow(clippy::wildcard_imports)]
use pebble_sys::{
	standard_c::memory::void,
	user_interface::window::{Window as sysWindow, WindowHandlers as sysWindowHandlers, *},
};
use unsafe_unwrap::UnsafeUnwrap;

pub mod number_window;

#[repr(transparent)] // Needed for WindowRef and WindowRefMut to work.
pub struct Window<T: ?Sized>(pub(crate) Handle<'static, sysWindow>, PhantomData<T>);

#[repr(transparent)]
pub struct WindowRef<'a>(Handle<'a, sysWindow>);

#[repr(transparent)]
pub struct WindowRefMut<'a>(Handle<'a, sysWindow>);

pub struct WindowHandlers<L: FnMut() -> T, A: FnMut(&mut T), D: FnMut(&mut T), U: FnMut(T), T> {
	pub load: L,
	pub appear: A,
	pub disappear: D,
	pub unload: U,
}

trait WindowHandlersTrait<T> {
	fn load(&mut self) -> T;
	fn appear(&mut self, data: &mut T);
	fn disappear(&mut self, data: &mut T);
	fn unload(&mut self, data: T);
}

impl<L: FnMut() -> T, A: FnMut(&mut T), D: FnMut(&mut T), U: FnMut(T), T> WindowHandlersTrait<T>
	for WindowHandlers<L, A, D, U, T>
{
	fn load(&mut self) -> T {
		(self.load)()
	}

	fn appear(&mut self, data: &mut T) {
		(self.appear)(data)
	}

	fn disappear(&mut self, data: &mut T) {
		(self.disappear)(data)
	}

	fn unload(&mut self, data: T) {
		(self.unload)(data)
	}
}

struct WindowData<'a, T> {
	user_data: Option<T>,
	window_handlers: Box<'a, dyn 'a + WindowHandlersTrait<T>>,
}

pub struct WindowCreationError<L: FnMut() -> T, A: FnMut(&mut T), D: FnMut(&mut T), U: FnMut(T), T>
{
	pub window_handlers: WindowHandlers<L, A, D, U, T>,
}

impl<T> Window<T> {
	/// Creates a new [`Window<T>`] instance with the specified [window handlers].
	///
	/// [`Window<T>`]: #
	/// [window handlers]: ./struct.WindowHandlers.html
	///
	/// # Errors
	///
	/// This function errors if associated data can't be allocated on the heap or if the window can't be created for another reason.
	pub fn new<
		'a,
		L: 'a + FnMut() -> T,
		A: 'a + FnMut(&mut T),
		D: 'a + FnMut(&mut T),
		U: 'a + FnMut(T),
	>(
		window_handlers: WindowHandlers<L, A, D, U, T>,
	) -> Result<Self, WindowCreationError<L, A, D, U, T>>
	where
		T: 'a,
	{
		#![allow(clippy::items_after_statements)]

		let window_data = Box::new(WindowData {
			user_data: None,
			window_handlers: Box::new(window_handlers)
				.map_err(|window_handlers| WindowCreationError { window_handlers })?,
		})
		.map_err(|window_data| WindowCreationError::<_, _, _, _, T> {
			window_handlers: Box::into_inner(unsafe {
				Box::downcast_unchecked(window_data.window_handlers)
			}),
		})?;
		let raw_window = match unsafe { window_create() } {
			Some(raw_window) => raw_window,
			None => {
				return Err(WindowCreationError {
					window_handlers: Box::into_inner(unsafe {
						Box::downcast_unchecked(Box::into_inner(window_data).window_handlers)
					}),
				});
			}
		};

		extern "C" fn raw_load<T>(raw_window: &mut sysWindow) {
			let window_data = unsafe {
				window_get_user_data(raw_window)
					.cast::<WindowData<T>>()
					.as_mut()
					.unsafe_unwrap()
			};
			window_data
				.user_data
				.replace(window_data.window_handlers.load())
				.debugless_unwrap_none();
		}
		extern "C" fn raw_appear<T>(raw_window: &mut sysWindow) {
			let window_data = unsafe {
				window_get_user_data(raw_window)
					.cast::<WindowData<T>>()
					.as_mut()
					.unsafe_unwrap()
			};
			window_data
				.window_handlers
				.appear(unsafe { window_data.user_data.as_mut().unsafe_unwrap() });
		}
		extern "C" fn raw_disappear<T>(raw_window: &mut sysWindow) {
			let window_data = unsafe {
				window_get_user_data(raw_window)
					.cast::<WindowData<T>>()
					.as_mut()
					.unsafe_unwrap()
			};
			window_data
				.window_handlers
				.disappear(unsafe { window_data.user_data.as_mut().unsafe_unwrap() });
		}
		extern "C" fn raw_unload<T>(raw_window: &mut sysWindow) {
			let window_data = unsafe {
				window_get_user_data(raw_window)
					.cast::<WindowData<T>>()
					.as_mut()
					.unsafe_unwrap()
			};
			window_data
				.window_handlers
				.unload(unsafe { window_data.user_data.take().unsafe_unwrap() });
		}

		unsafe {
			//SAFETY: window_data is only retrieved and destroyed in the destructor, *after* destroying the window.
			window_set_user_data(raw_window, {
				let mem: &mut void = Box::leak(window_data).into();
				mem
			});
			window_set_window_handlers(
				raw_window,
				sysWindowHandlers {
					load: Some(raw_load::<T>),
					appear: Some(raw_appear::<T>),
					disappear: Some(raw_disappear::<T>),
					unload: Some(raw_unload::<T>),
				},
			)
		}
		Ok(Self(Handle::new(raw_window), PhantomData))
	}

	/// Assembles a new instance of [`Window<T>`] from the given raw window handle.
	///
	/// [`Window<T>`]: #
	///
	/// # Safety
	///
	/// This function is only safe if `raw_window` is a raw window handle that was previously [`.leak()`]ed from the same [`Window<T>`] variant and no other [`Window<T>`] instance has been created from it since.
	///
	/// [`.leak()`]: #method.leak
	/// [`Window<T>`]: #
	pub unsafe fn from_raw(raw_window: &'static mut sysWindow) -> Self {
		Self(Handle::new(raw_window), PhantomData)
	}

	/// Leaks the current [`Window<T>`] instance into a raw Pebble window handle.
	///
	/// Note that [`Window<T>`] has associated heap instances beyond the raw window, so only destroying that would still leak memory.
	///
	/// [`Window<T>`]: #
	#[must_use = "Not reassembling the `Window<T>` later causes a memory leak."]
	pub fn leak(self) -> &'static mut sysWindow
	where
		T: 'static,
	{
		unsafe { ManuallyDrop::new(self).0.duplicate().unwrap() }
	}
}

impl<T: ?Sized> Window<T> {
	#[allow(clippy::must_use_candidate)] // side effects
	pub fn hide(&self, animated: bool) -> bool {
		window_stack::remove(self, animated)
	}

	#[must_use]
	pub fn is_loaded(&self) -> bool {
		unsafe { window_is_loaded(&*self.0) }
	}

	/// Pushes this window onto the window navidation stack, as topmost window of the app.
	///
	/// # Arguments
	///
	/// `animated`: Whether to animate the push using a sliding animation.
	pub fn show(&self, animated: bool) {
		window_stack::push(self, animated)
	}

	pub fn set_background_color(&self, background_color: Color8) {
		unsafe { window_set_background_color(self.0.as_mut_unchecked(), background_color) }
	}
}

impl<T: ?Sized> Drop for Window<T> {
	fn drop(&mut self) {
		self.special_drop()
	}
}

impl<T: ?Sized> SpecialDrop for Window<T> {
	default fn special_drop(&mut self) {
		panic!("Dropping unsized `Window<T>`s is illegal")
	}
}

impl<T: Sized> SpecialDrop for Window<T> {
	fn special_drop(&mut self) {
		unsafe {
			//SAFETY: window_data is created and leaked in the only accessible constructor.
			//SAFETY: self.0 isn't accessed after this.
			let window_data = window_get_user_data(&*self.0).cast();
			window_destroy(self.0.duplicate().unwrap());
			Box::<WindowData<T>>::from_raw(&mut *window_data);
		}
	}
}

impl<'a> Deref for WindowRef<'a> {
	type Target = Window<void>;

	fn deref(&self) -> &Self::Target {
		//SAFETY: Same memory layout, no access to data.
		unsafe { &*(self as *const _ as *const Window<void>) }
	}
}

impl<'a> Deref for WindowRefMut<'a> {
	type Target = Window<void>;

	fn deref(&self) -> &Self::Target {
		//SAFETY: Same memory layout, no access to data.
		unsafe { &*(self as *const _ as *const Window<void>) }
	}
}

impl<'a> DerefMut for WindowRefMut<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		//SAFETY: Same memory layout, no access to data.
		unsafe { &mut *(self as *mut _ as *mut Window<void>) }
	}
}
