mod egg_impls;
pub mod ffi;
mod macros;
mod network;
mod receiver;
mod rewrite;

pub use egg;
pub use libc;
pub use paste;
pub use seq_macro;

pub use crate::ffi::{FFIGate, ReceiverFFI};
pub use egg_impls::*;
pub use network::*;
pub use receiver::*;
pub use rewrite::*;

define_network! {
    pub enum Mig {
        "maj" = Maj(3)
    }
}

impl ReceiveFrom<FFIGate> for Mig {
    fn receive_from(from: FFIGate, receiver: &mut impl Receiver<Gate = Self>) -> Signal {
        match from {
            FFIGate::Maj(signals) => receiver.create(Mig::Maj(signals)),
            FFIGate::And(signals) => {
                let f = receiver.create_false();
                receiver.create(Mig::Maj([f, signals[0], signals[1]]))
            }
            FFIGate::Xor(signals) => {
                let f = receiver.create_false();
                let a = receiver.create(Mig::Maj([!signals[0], signals[1], f]));
                let b = receiver.create(Mig::Maj([signals[0], !signals[1], f]));
                receiver.create(Mig::Maj([!f, a, b]))
            }
            FFIGate::Xor3(signals) => {
                let a = receiver.create(Mig::Maj([signals[0], !signals[1], signals[2]]));
                let b = receiver.create(Mig::Maj([signals[0], signals[1], !signals[2]]));
                receiver.create(Mig::Maj([!signals[0], a, b]))
            }
        }
    }
}

define_network! {
    pub enum Aig {
        "and" = And(2)
    }
}

impl ReceiveFrom<FFIGate> for Aig {
    fn receive_from(from: FFIGate, receiver: &mut impl Receiver<Gate = Self>) -> Signal {
        let mut create_xor = |signals: [Signal; 2]| -> Signal {
            let a = receiver.create(Self::And([signals[0], !signals[1]]));
            let b = receiver.create(Self::And([!signals[0], signals[1]]));
            !receiver.create(Self::And([!a, !b]))
        };
        match from {
            FFIGate::And(signals) => receiver.create(Self::And(signals)),
            FFIGate::Xor(signals) => create_xor(signals),
            FFIGate::Xor3(signals) => {
                let a = create_xor([signals[1], signals[2]]);
                create_xor([signals[0], a])
            }
            FFIGate::Maj(signals) => {
                let a = receiver.create(Self::And([signals[0], signals[1]]));
                let b = receiver.create(Self::And([!signals[0], !signals[1]]));
                let c = receiver.create(Self::And([!b, signals[2]]));
                !receiver.create(Self::And([!a, !c]))
            }
        }
    }
}

define_network! {
    pub enum Xag {
        "and" = And(2),
        "xor" = Xor(2)
    }
}

impl ReceiveFrom<FFIGate> for Xag {
    fn receive_from(from: FFIGate, receiver: &mut impl Receiver<Gate = Self>) -> Signal {
        match from {
            FFIGate::And(signals) => receiver.create(Self::And(signals)),
            FFIGate::Xor(signals) => receiver.create(Self::Xor(signals)),
            FFIGate::Xor3(signals) => {
                let a = receiver.create(Self::Xor([signals[1], signals[2]]));
                receiver.create(Self::Xor([signals[0], a]))
            }
            FFIGate::Maj(signals) => {
                let a = receiver.create(Self::Xor([signals[0], signals[1]]));
                let b = receiver.create(Self::Xor([signals[0], signals[2]]));
                let c = receiver.create(Self::And([a, b]));
                receiver.create(Self::Xor([signals[0], c]))
            }
        }
    }
}

define_network! {
    pub enum Xmg {
        "xor" = Xor3(Xor, 3),
        "maj" = Maj(3)
    }
}

impl ReceiveFrom<FFIGate> for Xmg {
    fn receive_from(from: FFIGate, receiver: &mut impl Receiver<Gate = Self>) -> Signal {
        match from {
            FFIGate::Xor3(signals) => receiver.create(Self::Xor3(signals)),
            FFIGate::Maj(signals) => receiver.create(Self::Maj(signals)),
            FFIGate::And(signals) => {
                let f = receiver.create_false();
                receiver.create(Self::Maj([signals[0], signals[1], f]))
            }
            FFIGate::Xor(signals) => {
                let f = receiver.create_false();
                receiver.create(Self::Xor3([signals[0], signals[1], f]))
            }
        }
    }
}
