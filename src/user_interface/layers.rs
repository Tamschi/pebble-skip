use crate::Handle;
use core::marker::PhantomData;
#[allow(clippy::wildcard_imports)]
use pebble_sys::user_interface::layers::Layer as sysLayer;

pub struct Layer<T>(pub(crate) Handle<'static, sysLayer>, PhantomData<T>);
