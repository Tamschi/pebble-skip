use pebble_sys::standard_c::memory::c_str;

use crate::Box;
use core::{
	convert::{TryFrom, TryInto},
	marker::PhantomData,
	ops::{Deref, DerefMut},
	slice, str,
};

pub mod memory;

#[allow(non_camel_case_types)]
pub type void = pebble_sys::standard_c::memory::void;

/// A zero-terminated UTF-8 string slice.  
/// Note: When dereferencing this type to [`str`], the trailing `'\0'` is **not** included.
///
/// # Why this is reimplemented (aside from not being in [`core`]):
///
/// Comment on [`std::ffi::CStr`]:
/// ```
/// // Anyway, `CStr` representation and layout are considered implementation detail, are
/// // not documented and must not be relied upon.
/// ```
#[repr(transparent)]
pub struct CStr<T: Storage>(PhantomData<T>, str);

pub struct Heap(!);
pub struct Stack(!);
pub struct Static(!);

mod private {
	use super::{Heap, Stack, Static};

	pub trait Sealed {}
	impl Sealed for Heap {}
	impl Sealed for Stack {}
	impl Sealed for Static {}
}

pub trait Storage: private::Sealed {}
impl Storage for Heap {}
impl Storage for Stack {}
impl Storage for Static {}

pub trait NotStack: Storage {}
impl NotStack for Heap {}
impl NotStack for Static {}

trait AsCStr {
	type Storage: Storage;
	fn as_c_str(&self) -> Result<&CStr<Self::Storage>, ()>;
}

impl<'a> TryFrom<Box<'a, str>> for Box<'a, CStr<Heap>> {
	type Error = ();

	fn try_from(value: Box<str>) -> Result<Self, Self::Error> {
		match value.ends_with('\0') {
			true => {
				Ok(unsafe { Box::from_raw(&mut *(Box::leak(value) as *mut _ as *mut CStr<Heap>)) })
			}
			false => Err(()),
		}
	}
}

impl TryFrom<&'static str> for &'static CStr<Static> {
	type Error = ();

	fn try_from(value: &'static str) -> Result<Self, Self::Error> {
		match value.ends_with('\0') {
			true => Ok(unsafe { &*(value as *const _ as *const CStr<Static>) }),
			false => Err(()),
		}
	}
}

impl TryFrom<&'static mut str> for &'static mut CStr<Static> {
	type Error = ();

	fn try_from(value: &'static mut str) -> Result<Self, Self::Error> {
		match value.ends_with('\0') {
			true => Ok(unsafe { &mut *(value as *mut _ as *mut CStr<Static>) }),
			false => Err(()),
		}
	}
}

impl<'a> TryFrom<&'a str> for &'a CStr<Stack> {
	type Error = ();

	fn try_from(value: &'a str) -> Result<Self, Self::Error> {
		match value.ends_with('\0') {
			true => Ok(unsafe { &*(value as *const _ as *const CStr<Stack>) }),
			false => Err(()),
		}
	}
}

impl<'a> TryFrom<&'a mut str> for &'a mut CStr<Stack> {
	type Error = ();

	fn try_from(value: &'a mut str) -> Result<Self, Self::Error> {
		match value.ends_with('\0') {
			true => Ok(unsafe { &mut *(value as *mut _ as *mut CStr<Stack>) }),
			false => Err(()),
		}
	}
}

impl<'a> From<Box<'a, CStr<Heap>>> for Box<'a, str> {
	fn from(value: Box<'a, CStr<Heap>>) -> Self {
		unsafe { Box::from_raw(&mut *(Box::leak(value) as *mut _ as *mut str)) }
	}
}

impl<T: Storage> Deref for CStr<T> {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		let s = unsafe { &*(self as *const _ as *const str) };
		&s[..s.len() - 1]
	}
}

impl<T: Storage> DerefMut for CStr<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		let s = unsafe { &mut *(self as *mut _ as *mut str) };
		let len = s.len();
		&mut s[..len - 1]
	}
}

impl<T: Storage> CStr<T> {
	#[must_use]
	pub fn as_c_str(&self) -> &c_str {
		unsafe { &*(self as *const _ as *const c_str) }
	}

	#[must_use]
	pub fn as_c_str_mut(&mut self) -> &mut c_str {
		unsafe { &mut *(self as *mut _ as *mut c_str) }
	}
}

impl<T: Storage> CStr<T> {
	/// # Safety
	///
	/// As the name says, only if `slice` is zero-terminated and has [`Storage`] `T`.
	#[must_use]
	pub unsafe fn from_zero_terminated_unchecked(str: &str) -> &Self {
		&*(str as *const _ as *const CStr<T>)
	}

	/// # Safety
	///
	/// As the name says, only if `slice` is zero-terminated and has [`Storage`] `T`.
	#[must_use]
	pub unsafe fn from_zero_terminated_unchecked_mut(str: &mut str) -> &mut Self {
		&mut *(str as *mut _ as *mut CStr<T>)
	}

	/// # Safety
	///
	/// Only safe if `c_str` and `len` represent a valid zero-terminated UTF-8 string with [`Storage`] `T`.
	#[must_use]
	pub unsafe fn from_raw_parts_mut(c_str: &mut c_str, len: usize) -> &mut Self {
		let slice = slice::from_raw_parts_mut(c_str as *mut _ as *mut u8, len);
		let str = str::from_utf8_unchecked_mut(slice);
		&mut *(str as *mut _ as *mut CStr<T>)
	}
}

impl CStr<Stack> {
	/// # Errors
	///
	/// If `str` doesn't end with `'\0'`.
	pub fn try_from_stack(str: &str) -> Result<&Self, ()> {
		str.try_into()
	}
}

impl CStr<Static> {
	/// # Errors
	///
	/// If `str` doesn't end with `'\0'`.
	pub fn try_from_static(str: &'static str) -> Result<&'static Self, ()> {
		str.try_into()
	}

	/// # Safety
	///
	/// Safe iff str is a zero-terminated str with static storage.
	#[must_use]
	pub unsafe fn from_static_zero_terminated_unchecked(str: &'static str) -> &'static Self {
		CStr::<Static>::from_zero_terminated_unchecked(str)
	}
}
