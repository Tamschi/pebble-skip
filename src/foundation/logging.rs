use crate::standard_c::{CStr, Storage};
#[allow(clippy::wildcard_imports)]
use pebble_sys::foundation::logging::*;

pub fn log(
	log_level: u8,
	src_filename: &CStr<impl Storage>,
	src_line_number: i32,
	message: &CStr<impl Storage>,
) {
	//FIXME: make sure message doesn't contain any C format instructions.
	unsafe {
		app_log(
			log_level,
			src_filename.as_c_str(),
			src_line_number,
			message.as_c_str(),
		)
	}
}

#[macro_export]
macro_rules! log {
	($log_level:expr, $message:literal) => {
		$crate::foundation::logging::log(
			$log_level,
			unsafe {
				$crate::standard_c::CStr::from_static_zero_terminated_unchecked(concat!(
					file!(),
					"\0"
				))
				},
			line!() as i32,
			unsafe {
				$crate::standard_c::CStr::from_static_zero_terminated_unchecked(concat!(
					$message, "\0"
				))
				},
			)
	};
}
