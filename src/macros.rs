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
        $($gate_str:literal = $gate:ident($fanin:tt $(, $fn:ident)?)),+
    }) => {
        $crate::paste::paste! {
            $crate::egg::define_language! {
                $(#[$meta])*
                $vis enum [<$name Language>] {
                    Input(u32),
                    "f" = False,
                    "!" = Not($crate::egg::Id),
                    $($gate_str = $gate($crate::define_network!(@fanin_typ $crate::egg::Id, $fanin))),+,
                }
            }

            #[derive(Debug, Clone, Eq, PartialEq, Hash)]
            $(#[$meta])*
            $vis enum $name {
                $($gate($crate::define_network!(@fanin_typ $crate::Signal, $fanin))),+
            }

            impl $crate::Gate for $name {
                fn map_input_signals(mut self, mut map: impl FnMut(Signal) -> Signal) -> Self {
                    match &mut self {
                        $(Self::$gate(signals) => {
                            signals.iter_mut().for_each(|signal| *signal = map(*signal));
                        }),+
                    }
                    self
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
                            $crate::define_network!(@map_ids ids, signal_mapper, $fanin)
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
                            $crate::define_network!(@map_ids ids, id_mapper, $fanin)
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
    (@fanin_typ $of:ty, *) => {
        Vec<$of>
    };
    (@fanin_typ $of:ty, $num:literal) => {
        [$of; $num]
    };
    (@map_ids $ids:ident, $map:ident, *) => {
        // &mut to silence a warning for unused mut
        Vec::from_iter($ids.iter().copied().map(&mut $map))
    };
    (@map_ids $ids:ident, $map:ident, $num:literal) => {
        $crate::seq_macro::seq!(N in 0..$num {
            [#($map($ids[N]),)*]
        })
    }
}
