use core::marker::PhantomData;

#[cfg(feature = "_runtime")]
mod board;

#[cfg(feature = "_runtime")]
mod display;

pub struct Board<M> {
    _message: PhantomData<M>,
}
