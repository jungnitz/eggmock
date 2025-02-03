/// # Example
/// ```no_run
/// eggmock::define_network! {
///     pub enum "xag" = Xag {
///         // 2: fanin
///         // create_and: name of create method in mockturtle
///         // is_and: name of is method in mockturtle
///         "*" = And(2, create_and, is_and),
///         "xor" = Xor(2, create_xor, is_xor)
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_network {
    ($(#[$meta:meta])* $vis:vis enum $mockturtle_ntk:literal = $name:ident {
        $($gate_str:literal = $gate:ident($fanin:literal, $mockturtle_create:ident, $mockturtle_is:ident)),+
    }) => {
        $crate::paste::paste! {
            $crate::egg::define_language! {
                $(#[$meta])*
                $vis enum [<$name Language>] {
                    Input(u64),
                    "f" = False,
                    "!" = Not($crate::egg::Id),
                    $($gate_str = $gate([$crate::egg::Id;$fanin])),+,
                }
            }

            #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
            $vis enum [<$name Node>] {
                Input(u64),
                False,
                $($gate([$crate::Signal;$fanin])),+
            }

            $vis struct $name;

            impl $crate::Network for $name {
                type Node = [<$name Node>];
                type Gates = $crate::paste::paste!([<$name GateType>]);
                type Language = [<$name Language>];
                type ReceiverFFI<R> = [<$name ReceiverFFI>]<R>;

                const TYPENAME: &'static str = stringify!([<$name:snake:lower>]);
                const MOCKTURTLE_TYPENAME: &'static str = concat!($mockturtle_ntk, "_network");
                const MOCKTURTLE_INCLUDE: &'static str = concat!(
                    "mockturtle/networks/", $mockturtle_ntk, ".hpp"
                );
            }

            impl $crate::Node for [<$name Node>] {
                type Network = $name;

                fn map_input_signals(&self, mut map: impl FnMut(Signal) -> Signal) -> Self {
                    match self {
                        Self::Input(name) => Self::Input(*name),
                        Self::False => Self::False,
                        $(Self::$gate(signals) => {
                            $crate::seq_macro::seq!(N in 0..$fanin {
                                Self::$gate([#(map(signals[N]),)*])
                            })
                        }),+
                    }
                }

                fn inputs(&self) -> &[Signal] {
                    match self {
                        Self::Input(_) => &[],
                        Self::False => &[],
                        $(Self::$gate(ids) => ids),+
                    }
                }
            }

            impl NetworkLanguage for [<$name Language>] {
                type Network = $name;

                fn from_node(
                    node: [<$name Node>],
                    mut signal_mapper: impl FnMut(Signal) -> egg::Id,
                ) -> Self {
                    match node {
                        [<$name Node>]::Input(id) => Self::Input(id),
                        [<$name Node>]::False => Self::False,
                        $(
                        [<$name Node>]::$gate(ids) => Self::$gate(
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
                ) -> Option<[<$name Node>]> {
                    match self {
                        Self::Input(id) => Some([<$name Node>]::Input(*id)),
                        Self::False => Some([<$name Node>]::False),
                        Self::Not(_) => None,
                        $(
                        Self::$gate(ids) => Some([<$name Node>]::$gate(
                            $crate::seq_macro::seq!(N in 0..$fanin {
                                [#(id_mapper(ids[N]),)*]
                            })
                        ))
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

            #[derive(Copy, Clone, Eq, PartialEq, Hash)]
            $vis enum [<$name GateType>] {
                $($gate),+
            }

            impl $crate::GateType for [<$name GateType>] {
                type Network = $name;
                const VARIANTS: &'static [Self] = &[
                    $(Self::$gate),+
                ];

                fn name(&self) -> &'static str {
                    match self {
                        $(Self::$gate => stringify!([<$gate:snake:lower>])),+
                    }
                }

                fn fanin(&self) -> u8 {
                    match self {
                        $(Self::$gate => $fanin),+
                    }
                }

                fn mockturtle_create(&self) -> &'static str {
                    match self {
                        $(Self::$gate => stringify!($mockturtle_create)),+
                    }
                }

                fn mockturtle_is(&self) -> &'static str {
                    match self {
                        $(Self::$gate => stringify!($mockturtle_is)),+
                    }
                }
            }

            #[repr(C)]
            $vis struct [<$name ReceiverFFI>]<R> {
                data: *mut $crate::libc::c_void,
                create_input: extern "C" fn(*mut $crate::libc::c_void, name: u64) -> $crate::Signal,
                create_constant: extern "C" fn (*mut $crate::libc::c_void, value: bool) -> $crate::Signal,
                $([<create_ $gate:snake:lower>]: $crate::seq_macro::seq!(N in 1..=$fanin {
                     extern "C" fn(*mut $crate::libc::c_void, #(input~N: $crate::Signal,)*) -> $crate::Signal
                })),+,
                done: extern "C" fn (*mut $crate::libc::c_void, outputs: *const $crate::Signal, outputs_size: usize) -> R,
            }

            impl<R> $crate::ReceiverFFI for [<$name ReceiverFFI>]<R> {
                fn new<Recv>(receiver: Recv) -> Self
                where
                    Recv: $crate::Receiver<Node = [<$name Node>], Result = R> + 'static
                {
                    let data = Box::into_raw(Box::new(receiver));
                    Self {
                        data: data as *mut $crate::libc::c_void,
                        create_input: Self::create_input::<Recv>,
                        create_constant: Self::create_constant::<Recv>,
                        $([<create_ $gate:snake:lower>]: Self::[<create_ $gate:snake:lower>]::<Recv>),+,
                        done: Self::done::<Recv>,
                    }
                }
            }

            impl<R> $crate::Receiver for [<$name ReceiverFFI>]<R> {
                type Node = [<$name Node>];
                type Result = R;

                fn create_node(&mut self, node: [<$name Node>]) -> $crate::Signal {
                    match node {
                        [<$name Node>]::Input(name) => (self.create_input)(self.data, name),
                        [<$name Node>]::False => (self.create_constant)(self.data, false),
                        $([<$name Node>]::$gate(ids) => {
                            $crate::seq_macro::seq!(N in 0..$fanin {
                                (self.[<create_ $gate:snake:lower>])(self.data, #(ids[N],)*)
                            })
                        }),+
                    }
                }

                fn done(self, outputs: &[$crate::Signal]) -> R {
                    (self.done)(self.data, outputs.as_ptr(), outputs.len())
                }
            }

            impl<R> [<$name ReceiverFFI>]<R> {
                extern "C" fn create_input<Recv>(
                    data: *mut $crate::libc::c_void,
                    name: u64
                ) -> $crate::Signal
                where
                    Recv: $crate::Receiver<Node = [<$name Node>], Result = R> + 'static
                {
                    unsafe { &mut *(data as *mut Recv) }.create_node([<$name Node>]::Input(name))
                }

                extern "C" fn create_constant<Recv>(
                    data: *mut $crate::libc::c_void,
                    value: bool
                ) -> $crate::Signal
                where
                    Recv: $crate::Receiver<Node = [<$name Node>], Result = R> + 'static
                {
                    unsafe { &mut *(data as *mut Recv) }
                        .create_node([<$name Node>]::False)
                        .maybe_invert(value)
                }

                $($crate::seq_macro::seq!(N in 1..=$fanin {
                    pub extern "C" fn [<create_ $gate:snake:lower>]<Recv>(
                        data: *mut $crate::libc::c_void
                        #(, input~N: $crate::Signal)*
                    ) -> $crate::Signal
                    where
                        Recv: $crate::Receiver<Node = [<$name Node>], Result = R> + 'static
                    {
                        unsafe { &mut *(data as *mut Recv) }.create_node([<$name Node>]::$gate([#(input~N,)*]))
                    }
                });)+

                extern "C" fn done<Recv>(
                    data: *mut $crate::libc::c_void,
                    outputs: *const $crate::Signal,
                    outputs_size: usize,
                ) -> R
                where
                    Recv: $crate::Receiver<Node = [<$name Node>], Result = R> + 'static
                {
                    let outputs = if outputs_size == 0 {
                        &[]
                    } else {
                        unsafe { std::slice::from_raw_parts(outputs, outputs_size) }
                    };
                    unsafe { Box::from_raw(data as *mut Recv) }.done(outputs)
                }
            }
        }
    };
}
