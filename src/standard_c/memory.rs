use core::{
	cmp::Ordering,
	convert::TryInto,
	intrinsics::drop_in_place,
	mem::{needs_drop, size_of, size_of_val_raw, MaybeUninit},
	ptr::NonNull,
	slice,
};
use pebble_sys::{
	prelude::*,
	standard_c::memory::{self as sys_memory, void},
};

/// Allocates a heap memory slot for an instance of `T`.
///
/// The slot can safely be freed using [`pebble_sys::standard_c::memory::free`].
///
/// If `T` is zero-sized, then a valid slot is returned without allocation.
///
/// # Errors
///
/// If the allocation fails. Allocating for zero-sized `T` is infallible.
pub fn malloc<'a, T>() -> Result<&'a mut MaybeUninit<T>, ()> {
	match size_of::<T>() {
		0 => Ok(unsafe { &mut *(NonNull::dangling().as_ptr()) }),
		size => match unsafe { sys_memory::malloc(size).cast_unchecked_mut() } {
			Some(uninit) => Ok(uninit),
			None => Err(()),
		},
	}
}

/// Allocates a heap memory slot for `count` instances of `T`.
///
/// The slot can safely be freed using [`pebble_sys::standard_c::memory::free`].
///
/// If `T` is zero-sized, then a valid slot is returned without allocation.
///
/// # Errors
///
/// If the allocation fails. Allocating for zero-sized `T` is infallible.
pub fn calloc<'a, T>(count: usize) -> Result<&'a mut [MaybeUninit<T>], ()> {
	match size_of::<T>() {
		0 => Ok(unsafe { slice::from_raw_parts_mut(NonNull::dangling().as_ptr(), count) }),
		size => match unsafe {
			sys_memory::calloc(count, size)
				.map(|mem| slice::from_raw_parts_mut(mem.cast_unchecked_mut(), count))
		} {
			Some(uninit) => Ok(uninit),
			None => Err(()),
		},
	}
}

/// The result of a successful [`resize_realloc`] call.
pub enum ReallocOk<'a, T> {
	ShrunkenOrEqual(&'a mut [T]),
	Grown(&'a mut [MaybeUninit<T>]),
}

/// The result of a failed [`resize_realloc`] call.
pub enum ReallocError<'a, T> {
	CouldNotGrowOrMove(&'a mut [T]),
	CouldNotShrink(&'a mut [MaybeUninit<T>]),
}

/// Resizes a buffer, possibly moving it in the process. If `new_len < buffer.len()`, that is if the buffer is to be shrunk, any extra elements are dropped before this is attempted.
///
/// In cases where `buffer.len() == new_len`, both [`Ok`] and [`Err`] contain the `&mut [T]` variant.  
/// See [`ReallocOk`] and [`ReallocError`] for more information.
///
/// # Safety
///
/// This function is only safe iff `buffer` was obtained from an allocation function in this module.
///
/// # Errors
///
/// [`ReallocError`], iff the reallocation fails. If the buffer fails to grow, you get the original back.
///
/// If the buffer fails to shrink, you still get the original back, but any elements beyond the first `new_len` ones will have been dropped.
///
/// # Panics
///
/// If there is an arithmetic overflow regarding [`usize`] to [`isize`] conversions or pointer offset calculations.
#[allow(clippy::type_complexity)]
pub unsafe fn resize_realloc<'a, T>(
	buffer: &'static mut [T],
	new_len: usize,
) -> Result<ReallocOk<'a, T>, ReallocError<'a, T>> {
	let size = size_of::<T>();
	let old_len = buffer.len();
	let old_ptr = &mut buffer[0] as *mut T;
	if needs_drop::<T>() && new_len < old_len {
		for discarded in slice::from_raw_parts_mut(
			old_ptr.offset(new_len.try_into().expect("`new_len` overflow")),
			old_len - new_len,
		) {
			drop_in_place(discarded)
		}
	}
	match sys_memory::realloc(
		old_ptr as *mut _ as *mut void,
		size.checked_mul(new_len).expect("size overflow"),
	) {
		Some(new_ptr) => Ok(if old_len >= new_len {
			ReallocOk::ShrunkenOrEqual(slice::from_raw_parts_mut(
				new_ptr as *mut _ as *mut T,
				new_len,
			))
		} else {
			ReallocOk::Grown(slice::from_raw_parts_mut(
				new_ptr as *mut _ as *mut MaybeUninit<T>,
				new_len,
			))
		}),
		None => Err(if new_len >= old_len {
			ReallocError::CouldNotGrowOrMove(slice::from_raw_parts_mut(old_ptr, old_len))
		} else {
			ReallocError::CouldNotShrink(slice::from_raw_parts_mut(
				old_ptr as *mut _ as *mut MaybeUninit<T>,
				old_len,
			))
		}),
	}
}

/// Releases a heap memory slot after dropping the instance inside.
///
/// # Safety
///
/// This function is only safe if `slot` was obtained from an allocation function in this module.
pub unsafe fn drop_free<T: ?Sized>(slot: &'static mut T) {
	let ptr = slot as *mut T;
	drop_in_place(ptr);
	match size_of_val_raw(ptr) {
		0 => (),
		_ => sys_memory::free(&mut *(ptr as *mut _)),
	}
}

/// Compares two data slices.
///
/// # Panics
///
/// Iff `slice1.len() != slice2.len()`.
#[must_use]
#[track_caller]
pub fn memcmp(slice1: &[u8], slice2: &[u8]) -> Ordering {
	if slice1.len() != slice2.len() {
		panic!("Tried to memcmp slices of different sizes")
	}

	match unsafe { sys_memory::memcmp(slice1.upcast(), slice2.upcast(), slice1.len()) } {
		i32::MIN..=-1 => Ordering::Greater,
		0 => Ordering::Equal,
		1..=i32::MAX => Ordering::Less,
	}
}

/// # Safety
///
/// This is only safe with ![`Drop`] types.
pub unsafe fn memcpy<T>(dest: &mut [T], src: &[T]) {
	if src.len() != dest.len() {
		panic!("Tried to memcpy between slices of different sizes")
	}

	sys_memory::memcpy(
		(&mut *dest).upcast_mut(),
		src.upcast(),
		src.len() * size_of::<T>(),
	);
}

/// # Safety
///
/// This is only safe with ![`Drop`] types.
pub unsafe fn memcpy_uninit<T>(dest: &mut [MaybeUninit<T>], src: &[T]) {
	if src.len() != dest.len() {
		panic!("Tried to memcpy between slices of different sizes")
	}

	sys_memory::memcpy(
		(&mut *dest).upcast_mut(),
		src.upcast(),
		src.len() * size_of::<T>(),
	);
}

/// Sets all bytes in `dest` to `c`.
pub fn memset(dest: &mut [u8], c: u8) {
	let len = dest.len();
	unsafe { sys_memory::memset((&mut *dest).upcast_mut(), c.into(), len) };
}
