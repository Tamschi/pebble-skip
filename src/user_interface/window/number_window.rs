use crate::{
	standard_c::{CStr, NotStack},
	Box, Handle, SpecialDrop,
};
use core::{
	marker::PhantomData,
	mem::ManuallyDrop,
	ops::{Deref, DerefMut},
};
#[allow(clippy::wildcard_imports)]
use pebble_sys::{
	prelude::*,
	standard_c::memory::void,
	user_interface::window::number_window::{NumberWindow as sysNumberWindow, *},
};

use super::{WindowRef, WindowRefMut};

pub struct NumberWindow<'a, T: ?Sized>(
	pub(crate) Handle<'a, sysNumberWindow<'a>>,
	PhantomData<T>,
	*mut NumberWindowDataWrapper<'a>,
);

pub struct NumberWindowData<
	I: FnMut(&NumberWindow<void>, &mut T),
	D: FnMut(&NumberWindow<void>, &mut T),
	S: FnMut(&NumberWindow<void>, &mut T),
	T,
> {
	pub incremented: I,
	pub decremented: D,
	pub selected: S,
	pub context: T,
}

trait NumberWindowDataTrait {
	fn incremented(&mut self, number_window: &NumberWindow<void>);
	fn decremented(&mut self, number_window: &NumberWindow<void>);
	fn selected(&mut self, number_window: &NumberWindow<void>);
}

impl<
		I: FnMut(&NumberWindow<void>, &mut T),
		D: FnMut(&NumberWindow<void>, &mut T),
		S: FnMut(&NumberWindow<void>, &mut T),
		T,
	> NumberWindowDataTrait for NumberWindowData<I, D, S, T>
{
	fn incremented(&mut self, number_window: &NumberWindow<void>) {
		(self.incremented)(number_window, &mut self.context)
	}

	fn decremented(&mut self, number_window: &NumberWindow<void>) {
		(self.decremented)(number_window, &mut self.context)
	}

	fn selected(&mut self, number_window: &NumberWindow<void>) {
		(self.selected)(number_window, &mut self.context)
	}
}

pub struct NumberWindowDataWrapper<'a>(Box<'a, dyn 'a + NumberWindowDataTrait>);

impl<'a, T> NumberWindow<'a, T> {
	// TODO: This probably should take and set a set of window handlers, which can then also act as lifecycle hooks for the context.
	/// # Errors
	///
	/// TODO
	///
	pub fn new<
		I: 'a + FnMut(&NumberWindow<void>, &mut T),
		D: 'a + FnMut(&NumberWindow<void>, &mut T),
		S: 'a + FnMut(&NumberWindow<void>, &mut T),
	>(
		label: &'a CStr<impl NotStack>,
		number_window_data: NumberWindowData<I, D, S, T>,
	) -> Result<Self, NumberWindowData<I, D, S, T>>
	where
		T: 'a,
	{
		#![allow(clippy::items_after_statements)]

		let window_data_wrapper = Box::leak(
			Box::new(NumberWindowDataWrapper(Box::new(number_window_data)?)).map_err(
				|wrapper| Box::into_inner(unsafe { Box::downcast_unchecked(wrapper.0) }),
			)?,
		) as *mut NumberWindowDataWrapper;

		extern "C" fn raw_incremented<'a>(
			raw_window: &'a mut sysNumberWindow<'a>,
			context: &mut void,
		) {
			let context = context as *mut void; // This will be aliased.
			let fake_window = unsafe {
				//SAFETY: It's actually *kind of* safe to alias NumberWindow instances... But only because they store a Handle internally, which stores a pointer.
				// Actually accessing associated data would NOT be safe, so the user-provided handlers only see a NumberWindow<void> where such access is impossible.
				#[allow(clippy::cast_ptr_alignment)]
				NumberWindow::<void>::from_raw_unsized(
					raw_window,
					context as *mut _ as *mut NumberWindowDataWrapper,
				)
			};
			unsafe {
				//SAFETY: And here's the third concurrent use of this pointer.
				// The reference goes out of scope before the others are used, so this is safe.
				let context = &mut *context;
				context
					.cast_unchecked_mut::<NumberWindowDataWrapper>()
					.0
					.incremented(&fake_window)
			}
			fake_window.abandon();
		}
		extern "C" fn raw_decremented<'a>(
			raw_window: &'a mut sysNumberWindow<'a>,
			context: &mut void,
		) {
			let context = context as *mut void; // This will be aliased.
			let fake_window = unsafe {
				//SAFETY: It's actually *kind of* safe to alias NumberWindow instances... But only because they store a Handle internally, which stores a pointer.
				// Actually accessing associated data would NOT be safe, so the user-provided handlers only see a NumberWindow<void> where such access is impossible.
				#[allow(clippy::cast_ptr_alignment)]
				NumberWindow::<void>::from_raw_unsized(
					raw_window,
					context as *mut _ as *mut NumberWindowDataWrapper,
				)
			};
			unsafe {
				//SAFETY: And here's the third concurrent use of this pointer.
				// The reference goes out of scope before the others are used, so this is safe.
				let context = &mut *context;
				context
					.cast_unchecked_mut::<NumberWindowDataWrapper>()
					.0
					.decremented(&fake_window)
			}
			fake_window.abandon();
		}
		extern "C" fn raw_selected<'a>(
			raw_window: &'a mut sysNumberWindow<'a>,
			context: &mut void,
		) {
			let context = context as *mut void; // This will be aliased.
			let fake_window = unsafe {
				//SAFETY: It's actually *kind of* safe to alias NumberWindow instances... But only because they store a Handle internally, which stores a pointer.
				// Actually accessing associated data would NOT be safe, so the user-provided handlers only see a NumberWindow<void> where such access is impossible.
				#[allow(clippy::cast_ptr_alignment)]
				NumberWindow::<void>::from_raw_unsized(
					raw_window,
					context as *mut _ as *mut NumberWindowDataWrapper,
				)
			};
			unsafe {
				//SAFETY: And here's the third concurrent use of this pointer.
				// The reference goes out of scope before the others are used, so this is safe.
				let context = &mut *context;
				context
					.cast_unchecked_mut::<NumberWindowDataWrapper>()
					.0
					.selected(&fake_window)
			}
			fake_window.abandon();
		}

		match unsafe {
			number_window_create(
				label.as_c_str(),
				NumberWindowCallbacks {
					incremented: Some(raw_incremented),
					decremented: Some(raw_decremented),
					selected: Some(raw_selected),
				},
				&mut *(window_data_wrapper as *mut _ as *mut void),
			)
		} {
			Some(raw_window) => Ok(Self(
				Handle::new(raw_window),
				PhantomData,
				window_data_wrapper,
			)),
			None => Err(Box::into_inner(unsafe {
				Box::downcast_unchecked(
					Box::into_inner(Box::<NumberWindowDataWrapper>::from_raw(
						&mut *window_data_wrapper,
					))
					.0,
				)
			})),
		}
	}

	/// Assembles a new instance of [`NumberWindow`] from the given raw window handle.
	///
	/// # Safety
	///
	/// This function is only safe if `raw_window` is a raw window handle that was previously [`.leak()`]ed from the same [`NumberWindow`] variant and no other [`Window<T>`] instance has been created from it since.
	///
	/// [`.leak()`]: #method.leak
	pub unsafe fn from_raw(
		raw_window: &'a mut sysNumberWindow<'a>,
		number_window_data_wrapper: *mut NumberWindowDataWrapper<'a>,
	) -> Self {
		Self(
			Handle::new(raw_window),
			PhantomData,
			number_window_data_wrapper,
		)
	}

	/// Leaks the current [`NumberWindow`] instance into a raw Pebble number window handle.
	///
	/// Note that [`NumberWindow`] has associated heap instances beyond the raw window, so only destroying that would still leak memory.
	#[must_use = "Not reassembling the `NumberWindow` later causes a memory leak."]
	pub fn leak(
		self,
	) -> (
		&'a mut sysNumberWindow<'a>,
		*mut NumberWindowDataWrapper<'a>,
	)
	where
		T: 'a,
	{
		let undropped = ManuallyDrop::new(self);
		unsafe { (undropped.0.duplicate().unwrap(), undropped.2) }
	}
}

impl<'a, T: ?Sized> NumberWindow<'a, T> {
	#[must_use]
	pub fn window(&self) -> WindowRef<'_> {
		WindowRef(Handle::new(unsafe {
			number_window_get_window_mut(&mut *(self.0.as_mut_unchecked() as *mut _))
		}))
	}

	#[must_use]
	pub fn window_mut<'b: 'a>(&'b mut self) -> WindowRefMut<'b> {
		WindowRefMut(Handle::new(unsafe {
			number_window_get_window_mut(self.0.as_mut_unchecked())
		}))
	}

	pub fn set_label(&self, label: &'a CStr<impl NotStack>) {
		unsafe { number_window_set_label(self.0.as_mut_unchecked(), label.as_c_str()) }
	}

	pub fn set_max(&self, max: i32) {
		unsafe { number_window_set_max(self.0.as_mut_unchecked(), max) }
	}

	pub fn set_min(&self, min: i32) {
		unsafe { number_window_set_min(self.0.as_mut_unchecked(), min) }
	}

	pub fn set_value(&self, value: i32) {
		unsafe { number_window_set_value(self.0.as_mut_unchecked(), value) }
	}

	pub fn set_step_size(&self, step_size: i32) {
		unsafe { number_window_set_step_size(self.0.as_mut_unchecked(), step_size) }
	}

	#[must_use]
	pub fn get_value(&self) -> i32 {
		unsafe { number_window_get_value(&*self.0) }
	}

	/// # Safety
	///
	/// It's actually safe to assemble [`NumberWindow`] instances with mismatched type parameters iff the type parameter assembled against is unsized,
	/// because this data can never be directly accessed outside the destructor.
	///
	/// However, dropping such a value will always panic, so adding this to the public API would be a *really* bad idea.
	unsafe fn from_raw_unsized(
		raw_window: &'a mut sysNumberWindow<'a>,
		number_window_data_wrapper: *mut NumberWindowDataWrapper<'a>,
	) -> Self {
		Self(
			Handle::new(raw_window),
			PhantomData,
			number_window_data_wrapper,
		)
	}

	/// Discards this instance while skipping the destructor. Helper for aliased temporaries.
	fn abandon(self) {
		let _ = ManuallyDrop::new(self);
	}
}

impl<'a, T> Deref for NumberWindow<'a, T> {
	type Target = NumberWindow<'a, void>;

	fn deref(&self) -> &Self::Target {
		unsafe {
			//SAFETY: Same memory layout, no access to data.
			&*(self as *const _ as *const Self::Target)
		}
	}
}

impl<'a, T> DerefMut for NumberWindow<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe {
			//SAFETY: Same memory layout, no access to data.
			&mut *(self as *mut _ as *mut Self::Target)
		}
	}
}

impl<'a, T: ?Sized> Drop for NumberWindow<'a, T> {
	fn drop(&mut self) {
		self.special_drop()
	}
}

impl<'a, T: ?Sized> SpecialDrop for NumberWindow<'a, T> {
	default fn special_drop(&mut self) {
		panic!("Dropping unsized `NumberWindow<T>`s is illegal")
	}
}

impl<'a, T: Sized> SpecialDrop for NumberWindow<'a, T> {
	fn special_drop(&mut self) {
		unsafe {
			//SAFETY: window_data is created and leaked in the only accessible constructor.
			//SAFETY: self.0 isn't accessed after this.
			let data_wrapper = self.2;
			// Detaching the lifetime here takes a bit of work.
			let sys_number_window = self.0.duplicate().unwrap() as *mut _ as *mut void as *mut _;

			// Destroy the window, THEN drop its data.
			number_window_destroy(&mut *sys_number_window);
			Box::<NumberWindowDataWrapper>::from_raw(&mut *data_wrapper);
		}
	}
}
