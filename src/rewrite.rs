use crate::{
    Gate, ReceiveInto, Receiver,
    ffi::{FFIGate, ReceiverFFI},
};

/// Allows rewriting of logic networks.
///
/// The rewriting process works as follows:
/// 1. create a [`Receiver`]
/// 2. send the network to the receiver
/// 3. perform the rewrite on the result of the [`Receiver`]
/// 4. send the result of the rewrite to an output [`Receiver`]
pub trait Rewriter {
    type Gate: Gate;
    type Intermediate;

    fn create_receiver(
        &mut self,
    ) -> impl Receiver<Gate = Self::Gate, Result = Self::Intermediate> + 'static;
    fn rewrite(
        self,
        input: Self::Intermediate,
        output: impl Receiver<Gate = Self::Gate, Result = ()>,
    );
}

/// A struct that contains a data pointer and a function pointing to the function that performs the
/// rewrite using the data.
///
/// Allocated memory is released after a call to the rewrite function.
#[repr(C)]
pub struct RewriterFFI {
    data: *mut libc::c_void,
    free: extern "C" fn(*mut libc::c_void),
    rewrite: extern "C" fn(*mut libc::c_void, ReceiverFFI<()>),
}

impl RewriterFFI {
    pub fn new<'r, R>(mut rewriter: R) -> ReceiverFFI<'r, RewriterFFI>
    where
        R: Rewriter + 'r,
        R::Gate: ReceiveInto<FFIGate>,
        FFIGate: ReceiveInto<R::Gate>,
        R::Intermediate: 'static,
    {
        ReceiverFFI::new(rewriter.create_receiver().map(|result| {
            let data = Box::into_raw(Box::new((rewriter, result)));
            RewriterFFI {
                data: data as *mut libc::c_void,
                free: Self::free::<R>,
                rewrite: Self::rewrite::<R>,
            }
        }))
    }

    extern "C" fn free<R: Rewriter>(data: *mut libc::c_void) {
        let _: Box<(R, R::Intermediate)> =
            unsafe { Box::from_raw(data as *mut (R, R::Intermediate)) };
    }

    extern "C" fn rewrite<R: Rewriter>(data: *mut libc::c_void, callback: ReceiverFFI<()>)
    where
        R::Gate: ReceiveInto<FFIGate>,
    {
        let data = unsafe { Box::from_raw(data as *mut (R, R::Intermediate)) };
        data.0.rewrite(data.1, callback.with_input::<R::Gate>())
    }
}
