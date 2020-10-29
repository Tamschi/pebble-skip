#![no_std]
#![feature(coerce_unsized)]
#![feature(layout_for_ptr)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_ref)]
#![feature(maybe_uninit_slice)]
#![feature(min_specialization)]
#![feature(never_type)]
#![feature(unsize)]
#![warn(clippy::pedantic)]
#![allow(clippy::match_bool)]
#![allow(clippy::module_name_repetitions)] // Matching the SDK documentation.

use core::{
	future::Future,
	intrinsics::drop_in_place,
	marker::{PhantomData, Unsize},
	mem::{size_of_val_raw, ManuallyDrop, MaybeUninit},
	ops::{CoerceUnsized, Deref, DerefMut},
	pin::Pin,
	str,
	task::{Context, Poll},
};
use pebble_sys::standard_c::memory::free;
use standard_c::{
	memory::{calloc, malloc, memcpy_uninit},
	CStr, Heap,
};

pub mod foundation;
pub mod graphics;
pub mod standard_c;
pub mod user_interface;

trait SpecialDrop {
	fn special_drop(&mut self);
}

/// Just a standard Box, more or less. The main difference is that its constructor is fallible instead of panicking.
///
/// It probably has fewer features than Rust's version, but it should be possible to add or emulate those.
pub struct Box<'a, T: ?Sized>(&'a mut T);

impl<'a, T> Box<'a, T> {
	/// Moves `value` onto the Pebble heap.
	///
	/// # Errors
	///
	/// Iff the heap allocation fails.
	pub fn new(value: T) -> Result<Self, T> {
		match malloc::<T>() {
			Ok(uninit) => Ok(Self(uninit.write(value))),
			Err(()) => Err(value),
		}
	}

	/// Moves `r#box`'s value off the Pebble heap.
	#[must_use]
	pub fn into_inner(r#box: Self) -> T {
		let value;
		unsafe {
			let mem = Box::leak(r#box) as *mut T;
			value = mem.read();
			free(&mut *(mem as *mut _));
		}
		value
	}
}

impl<'a, T: ?Sized> Drop for Box<'a, T> {
	fn drop(&mut self) {
		unsafe {
			//SAFETY: ptr is always a valid pointer here that originally belonged to a sized type.
			let ptr = self.0 as *mut T;
			drop_in_place(ptr);
			match size_of_val_raw(ptr) {
				0 => (),
				_ => free(&mut *(ptr as *mut _)),
			};
		}
	}
}

impl<'a, T: ?Sized> Box<'a, T> {
	#[must_use = "If the Box instance is not reassembled later, a memory leak occurs."]
	pub fn leak(r#box: Self) -> &'a mut T
	where
		T: 'a,
	{
		unsafe { &mut *(ManuallyDrop::new(r#box).deref_mut().0 as *mut T) }
	}

	/// Reassembles a [`Box`] instance from a leaked reference.
	///
	/// # Safety
	///
	/// Iff the reference was previously leaked from a matching [`Box`] instance.
	pub unsafe fn from_raw(raw: &'a mut T) -> Self {
		Self(raw)
	}

	/// Reinterprets a [`Box`] of an type `T` into its original sized type `Box<U>`.
	///
	/// # Safety
	///
	/// Iff this instance was created from a value memory-compatible to `U`.
	#[must_use]
	pub unsafe fn downcast_unchecked<U: Unsize<T>>(r#box: Self) -> Box<'a, U> {
		Box::from_raw(&mut *(Box::leak(r#box) as *mut _ as *mut U))
	}
}

impl<'a, T: ?Sized> Deref for Box<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
	}
}

impl<'a, T: ?Sized> DerefMut for Box<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.0
	}
}

impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<'a, U>> for Box<'a, T> {}

impl<'a, T: ?Sized> Unpin for Box<'a, T> {}

impl<'a, F: ?Sized + Future + Unpin> Future for Box<'a, F> {
	type Output = F::Output;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		F::poll(Pin::new(&mut *self), cx)
	}
}

impl Box<'static, CStr<Heap>> {
	/// Clones a Rust [`str`] onto the Pebble heap, appending `'\0'` in the process.
	///
	/// # Errors
	///
	/// Iff the heap allocation fails.
	pub fn clone_to_c_str(value: &str) -> Result<Self, ()> {
		let mem = calloc::<u8>(value.len() + 1)?;
		unsafe {
			memcpy_uninit(&mut mem[..value.len()], value.as_bytes());
			mem[value.len()].write(0);
			let slice = MaybeUninit::slice_assume_init_mut(mem);
			let str = str::from_utf8_unchecked_mut(slice);
			let c_str = CStr::from_zero_terminated_unchecked_mut(str);
			Ok(Self::from_raw(c_str))
		}
	}
}

/// This is *sort of* like a Cell, but for constant handles. It should still allow surface-level aliasing.
///
/// Note that this is a reference wrapper and does not drop its target!
struct Handle<'a, T: 'a + ?Sized>(*mut T, PhantomData<&'a mut T>);

impl<'a, T: 'a + ?Sized> Handle<'a, T> {
	pub fn new(exclusive_handle: &'a mut T) -> Self {
		Self(exclusive_handle as *mut T, PhantomData)
	}

	pub fn unwrap(self) -> &'a mut T {
		unsafe { &mut *self.0 }
	}

	#[allow(clippy::mut_from_ref)]
	pub unsafe fn as_mut_unchecked(&self) -> &mut T {
		&mut *self.0
	}

	pub unsafe fn duplicate(&self) -> Self {
		Self(self.0, self.1)
	}
}

impl<'a, T: ?Sized> Deref for Handle<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.0 }
	}
}

impl<'a, T: ?Sized> DerefMut for Handle<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.0 }
	}
}
