use crate::Handle;
use core::marker::PhantomData;
#[allow(clippy::wildcard_imports)]
use pebble_sys::user_interface::layers::Layer as sysLayer;

//TODO
pub struct Layer<'a, T: ?Sized>(pub Handle<'a, sysLayer>, pub PhantomData<T>);
