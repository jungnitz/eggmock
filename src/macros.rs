/// # Example
/// ```no_run
/// eggmock::define_network! {
///     pub enum Xag {
///         // 2: fanin
///         "*" = And(2),
///         "xor" = Xor(2)
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_network {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($gate_str:literal = $gate:ident($($fn:ident,)? $fanin:literal)),+
    }) => {
        $crate::paste::paste! {
            $crate::egg::define_language! {
                $(#[$meta])*
                $vis enum [<$name Language>] {
                    Input(u32),
                    "f" = False,
                    "!" = Not($crate::egg::Id),
                    $($gate_str = $gate([$crate::egg::Id;$fanin])),+,
                }
            }

            #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
            $(#[$meta])*
            $vis enum $name {
                $($gate([$crate::Signal;$fanin])),+
            }

            impl $crate::Gate for $name {
                type Language = [<$name Language>];

                fn map_input_signals(self, mut map: impl FnMut(Signal) -> Signal) -> Self {
                    match self {
                        $(Self::$gate(signals) => {
                            $crate::seq_macro::seq!(N in 0..$fanin {
                                Self::$gate([#(map(signals[N]),)*])
                            })
                        }),+
                    }
                }

                fn inputs(&self) -> &[Signal] {
                    match self {
                        $(Self::$gate(ids) => ids),+
                    }
                }
            }

            impl $crate::ReceiveInto<$crate::FFIGate> for $name {
                fn receive_into(self, receiver: &mut impl Receiver<Gate = FFIGate>) -> Signal {
                    match self {
                        $(
                        Self::$gate(signals) => ffi::__private::receive_with_function(
                            receiver,
                            $crate::define_network!(@gate_fn $gate $($fn)?),
                            &signals,
                        ),
                        )*
                    }
                }
            }

            impl NetworkLanguage for [<$name Language>] {
                type Gate = $name;

                fn from_node(
                    node: Node<$name>,
                    mut signal_mapper: impl FnMut(Signal) -> egg::Id,
                ) -> Self {
                    match node {
                        $crate::Node::Input(id) => Self::Input(id),
                        $crate::Node::False => Self::False,
                        $(
                        $crate::Node::Gate($name::$gate(ids)) => Self::$gate(
                            $crate::seq_macro::seq!(N in 0..$fanin {
                                [#(signal_mapper(ids[N]),)*]
                            })
                        )
                        ),+
                    }
                }

                fn to_node(
                    &self,
                    mut id_mapper: impl FnMut(egg::Id) -> Signal
                ) -> Option<Node<$name>> {
                    match self {
                        Self::Input(id) => Some($crate::Node::Input(*id)),
                        Self::False => Some($crate::Node::False),
                        Self::Not(_) => None,
                        $(
                        Self::$gate(ids) => Some($crate::Node::Gate($name::$gate(
                            $crate::seq_macro::seq!(N in 0..$fanin {
                                [#(id_mapper(ids[N]),)*]
                            })
                        )))
                        ),+
                    }
                }

                fn is_not(&self) -> bool {
                    match self {
                        Self::Not(_) => true,
                        _ => false,
                    }
                }
                fn not(id: $crate::egg::Id) -> Self {
                    Self::Not(id)
                }
            }
        }
    };
    (@gate_fn $gate:ident $fn:ident) => {
        $crate::ffi::__private::GateFunction::$fn
    };
    (@gate_fn $gate:ident) => {
        $crate::ffi::__private::GateFunction::$gate
    };
}
