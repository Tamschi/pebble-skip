use core::marker::PhantomData;

#[allow(clippy::wildcard_imports)]
use pebble_sys::user_interface::vibes::{VibePattern as sysVibePattern, *};

pub fn cancel() {
	unsafe { vibes_cancel() }
}

pub fn short_pulse() {
	unsafe { vibes_short_pulse() }
}

pub fn long_pulse() {
	unsafe { vibes_long_pulse() }
}

pub fn double_pulse() {
	unsafe { vibes_double_pulse() }
}

pub fn enqueue_custom_pattern(pattern: &VibePattern) {
	unsafe { vibes_enqueue_custom_pattern(pattern.as_sys()) }
}

pub struct VibePattern<'a>(&'a [u32]);

pub enum VibePatternError {
	Empty,
	TooManyDurations,
	DurationTooLong { index: usize },
}

impl<'a> VibePattern<'a> {
	/// # Errors
	///
	/// If durations is empty, contains an element larger than `10_000`ms or
	/// its length doesn't fit 32 bit (but good luck allocating that on a Pebble 😉).
	pub fn new(durations: &'a [u32]) -> Result<Self, VibePatternError> {
		if durations.is_empty() {
			return Err(VibePatternError::Empty);
		}

		if durations.len() > u32::MAX as usize {
			return Err(VibePatternError::TooManyDurations);
		}

		for (index, duration) in durations.iter().enumerate() {
			if *duration > 10_000 {
				return Err(VibePatternError::DurationTooLong { index });
			}
		}

		Ok(Self(durations))
	}

	#[must_use]
	pub fn as_sys(&self) -> sysVibePattern<'a> {
		#[allow(clippy::cast_possible_truncation)] // Checked in constructor.
		sysVibePattern {
			durations: self.0 as *const _ as *const u32,
			num_segments: self.0.len() as u32,
			phantom: PhantomData,
		}
	}
}
