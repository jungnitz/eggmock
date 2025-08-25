use std::{marker::PhantomData, ptr::null_mut};

use crate::{Node, ReceiveInto, Receiver, Signal};

pub enum FFIGate {
    And([Signal; 2]),
    Xor([Signal; 2]),
    Xor3([Signal; 3]),
    Maj([Signal; 3]),
}

#[derive(Debug)]
#[repr(C)]
pub struct ReceiverFFI<'s, Res> {
    state: *mut libc::c_void,
    free: unsafe extern "C" fn(state: *mut libc::c_void),
    create_false: unsafe extern "C" fn(state: *mut libc::c_void) -> Signal,
    create_input: unsafe extern "C" fn(state: *mut libc::c_void, idx: u32) -> Signal,
    create_and: unsafe extern "C" fn(state: *mut libc::c_void, in0: Signal, in1: Signal) -> Signal,
    create_xor: unsafe extern "C" fn(state: *mut libc::c_void, in0: Signal, in1: Signal) -> Signal,
    create_xor3: unsafe extern "C" fn(
        state: *mut libc::c_void,
        in0: Signal,
        in1: Signal,
        in2: Signal,
    ) -> Signal,
    create_maj: unsafe extern "C" fn(
        state: *mut libc::c_void,
        in0: Signal,
        in1: Signal,
        in2: Signal,
    ) -> Signal,
    done: unsafe extern "C" fn(
        state: *mut libc::c_void,
        outputs: *const Signal,
        n_outputs: libc::size_t,
    ) -> Res,
    _state: PhantomData<&'s mut ()>,
}

impl<'s, Res> ReceiverFFI<'s, Res> {
    pub fn new<G, R: 's + Receiver<Gate = G, Result = Res>>(receiver: R) -> Self
    where
        FFIGate: ReceiveInto<G>,
    {
        Self::new_inner(receiver.with_input::<FFIGate>())
    }
    fn new_inner<R: 's + Receiver<Gate = FFIGate, Result = Res>>(receiver: R) -> Self {
        Self {
            state: Box::into_raw(Box::new(receiver)) as *mut libc::c_void,
            _state: PhantomData,
            free: Self::free::<R>,
            create_false: Self::create_false::<R>,
            create_input: Self::create_input::<R>,
            create_and: Self::create_and::<R>,
            create_xor: Self::create_xor::<R>,
            create_xor3: Self::create_xor3::<R>,
            create_maj: Self::create_maj::<R>,
            done: Self::done::<R>,
        }
    }

    unsafe extern "C" fn free<R: Receiver<Gate = FFIGate, Result = Res>>(state: *mut libc::c_void) {
        let _: Box<R> = unsafe { Box::from_raw(state as *mut R) };
    }

    unsafe extern "C" fn create_false<R: Receiver<Gate = FFIGate, Result = Res>>(
        state: *mut libc::c_void,
    ) -> Signal {
        (unsafe { &mut *(state as *mut R) }).create(Node::False)
    }

    unsafe extern "C" fn create_input<R: Receiver<Gate = FFIGate, Result = Res>>(
        state: *mut libc::c_void,
        idx: u32,
    ) -> Signal {
        (unsafe { &mut *(state as *mut R) }).create(Node::Input(idx))
    }

    unsafe extern "C" fn create_and<R: Receiver<Gate = FFIGate, Result = Res>>(
        state: *mut libc::c_void,
        in0: Signal,
        in1: Signal,
    ) -> Signal {
        (unsafe { &mut *(state as *mut R) }).create_gate(FFIGate::And([in0, in1]))
    }

    unsafe extern "C" fn create_xor<R: Receiver<Gate = FFIGate, Result = Res>>(
        state: *mut libc::c_void,
        in0: Signal,
        in1: Signal,
    ) -> Signal {
        (unsafe { &mut *(state as *mut R) }).create_gate(FFIGate::Xor([in0, in1]))
    }

    unsafe extern "C" fn create_xor3<R: Receiver<Gate = FFIGate, Result = Res>>(
        state: *mut libc::c_void,
        in0: Signal,
        in1: Signal,
        in2: Signal,
    ) -> Signal {
        (unsafe { &mut *(state as *mut R) }).create_gate(FFIGate::Xor3([in0, in1, in2]))
    }

    unsafe extern "C" fn create_maj<R: Receiver<Gate = FFIGate, Result = Res>>(
        state: *mut libc::c_void,
        in0: Signal,
        in1: Signal,
        in2: Signal,
    ) -> Signal {
        (unsafe { &mut *(state as *mut R) }).create_gate(FFIGate::Maj([in0, in1, in2]))
    }

    unsafe extern "C" fn done<R: Receiver<Gate = FFIGate, Result = Res>>(
        state: *mut libc::c_void,
        outputs: *const Signal,
        n_outputs: libc::size_t,
    ) -> Res {
        unsafe {
            Box::from_raw(state as *mut R)
                .done(Vec::from(std::slice::from_raw_parts(outputs, n_outputs)))
        }
    }
}

impl<'s, Res> Drop for ReceiverFFI<'s, Res> {
    fn drop(&mut self) {
        if !self.state.is_null() {
            unsafe { (self.free)(self.state) }
        }
    }
}

impl<'s, Res> Receiver for ReceiverFFI<'s, Res> {
    type Gate = FFIGate;
    type Result = Res;

    fn create(&mut self, node: Node<Self::Gate>) -> Signal {
        unsafe {
            match node {
                Node::False => (self.create_false)(self.state),
                Node::Input(idx) => (self.create_input)(self.state, idx),
                Node::Gate(gate) => match gate {
                    FFIGate::And(signals) => (self.create_and)(self.state, signals[0], signals[1]),
                    FFIGate::Xor(signals) => (self.create_xor)(self.state, signals[0], signals[1]),
                    FFIGate::Xor3(signals) => {
                        (self.create_xor3)(self.state, signals[0], signals[1], signals[2])
                    }
                    FFIGate::Maj(signals) => {
                        (self.create_maj)(self.state, signals[0], signals[1], signals[2])
                    }
                },
            }
        }
    }

    fn done(mut self, outputs: Vec<Signal>) -> Res {
        let res = unsafe { (self.done)(self.state, outputs.as_ptr(), outputs.len()) };
        self.state = null_mut();
        res
    }
}

#[doc(hidden)]
pub mod __private {
    use crate::GateFunction;

    use super::*;

    pub fn receive_with_function(
        receiver: &mut impl Receiver<Gate = FFIGate>,
        function: GateFunction,
        inputs: &[Signal],
    ) -> Signal {
        match function {
            GateFunction::And => treeify_signals::<2>(inputs, |signals| {
                receiver.create_gate(FFIGate::And(
                    <[Signal; 2]>::try_from(signals).expect("should be exactly 2 signals"),
                ))
            }),
            GateFunction::Xor => treeify_signals::<3>(inputs, |signals| {
                if let Ok(signals) = <[Signal; 3]>::try_from(signals) {
                    receiver.create_gate(FFIGate::Xor3(signals))
                } else if let Ok(signals) = <[Signal; 2]>::try_from(signals) {
                    receiver.create_gate(FFIGate::Xor(signals))
                } else {
                    panic!("should be either 2 or 3 signals")
                }
            }),
            GateFunction::Maj => treeify_signals::<3>(inputs, |signals| {
                receiver.create_gate(FFIGate::Maj(
                    <[Signal; 3]>::try_from(signals).expect("should be exactly 3 signals"),
                ))
            }),
        }
    }

    fn treeify_signals<const MAX_MUNCH: usize>(
        signals: &[Signal],
        mut primitive: impl FnMut(&[Signal]) -> Signal,
    ) -> Signal {
        // fast path without allocation
        if signals.len() == 1 {
            return signals[0];
        } else if signals.len() <= MAX_MUNCH {
            return primitive(signals);
        }

        let mut next = Vec::from(signals);
        loop {
            if next.len() == 1 {
                break next[0];
            }
            if next.len() <= MAX_MUNCH {
                break primitive(&next);
            }

            let mut start = 0;
            let mut i = 0;
            while start + MAX_MUNCH < next.len() {
                next[i] = primitive(&next[start..start + MAX_MUNCH]);
                i += 1;
                start += MAX_MUNCH;
            }
            for remaining_idx in start..next.len() {
                next[i] = next[remaining_idx];
                i += 1;
            }
            next.truncate(i);
        }
    }
}
