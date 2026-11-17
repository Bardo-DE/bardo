//! `services!`: the one declaration of every service. From each `field: Service` line it writes the
//! [`Services`](super::Services) field, the [`Member`](super::Member) impl and, behind `harness`,
//! the fake channel. A service is added here and in `shell-app`'s `start`, whose struct literal
//! does not compile while a field is missing.

/// The state a fake service starts at: the one given with `=>`, else its default.
#[cfg(feature = "harness")]
macro_rules! fake_state {
    ($service:ty) => {
        <<$service as shell_model::service::Service>::State as Default>::default()
    };
    ($service:ty, $fake:expr) => {
        $fake
    };
}

/// Declare the services, in the order they start.
///
/// ```ignore
/// services! {
///     {
///         /// Docs of the field.
///         battery: BatteryService;                            // the fake starts at its default
///         volume: VolumeService => fake_volume();             // the fake starts at this state
///     }
///     {
///         dirs: Dirs => Dirs::under(..);   // the fields that are not services, and their fakes
///     }
/// }
/// ```
macro_rules! services {
    (
        {
            $(
                $(#[$meta:meta])*
                $field:ident: $service:ty $(=> $fake:expr)?;
            )*
        }
        {
            $(
                $(#[$xmeta:meta])*
                $xfield:ident: $xty:ty => $xfake:expr;
            )*
        }
    ) => {
        /// Every running service, each field named after its module.
        ///
        /// Surfaces read a service through `shell-ui-kit::hooks::use_service` and command it
        /// through `use_control`; nothing here is a Dioxus signal (review rule: never pass a Signal
        /// through `Root::context`). The daemon builds it (`shell-app`'s `start`).
        #[derive(Debug, Clone)]
        pub struct Services {
            $(
                $(#[$meta])*
                pub $field: shell_model::service::Handle<$service>,
            )*
            $(
                $(#[$xmeta])*
                pub $xfield: $xty,
            )*
        }

        $(
            impl super::Member for $service {
                fn of(services: &Services) -> &shell_model::service::Handle<Self> {
                    &services.$field
                }
            }
        )*

        impl Services {
            /// Each declared service as `(field, name)`, in start order.
            pub const DECLARED: &'static [(&'static str, &'static str)] =
                &[$((stringify!($field), <$service as shell_model::service::Service>::NAME)),*];

            /// Each service this value holds, as `(field, name)`: [`Self::DECLARED`] when every
            /// declared service was started or faked.
            pub fn held(&self) -> Vec<(&'static str, &'static str)> {
                vec![$({
                    let _held: &shell_model::service::Handle<$service> = &self.$field;
                    (stringify!($field), <$service as shell_model::service::Service>::NAME)
                }),*]
            }

            /// Every service as a fake: its channel at the state given with `=>` (else its
            /// default), and the service's halves of the channels, for a test to claim.
            #[cfg(feature = "harness")]
            pub(crate) fn fake() -> (Services, Ios) {
                struct Pairs {
                    $($field: (
                        shell_model::service::Handle<$service>,
                        shell_model::service::ServiceIo<$service>,
                    ),)*
                }
                let pairs = Pairs {
                    $($field: shell_model::service::Handle::<$service>::channel(
                        fake_state!($service $(, $fake)?),
                    ),)*
                };
                let services = Services {
                    $($field: pairs.$field.0,)*
                    $($xfield: $xfake,)*
                };
                let ios = Ios {
                    $($field: super::fake::Claim::new(pairs.$field.1),)*
                };
                (services, ios)
            }
        }

        /// The service halves of every fake channel.
        #[cfg(feature = "harness")]
        pub(crate) struct Ios {
            $(pub(crate) $field: super::fake::Claim<$service>,)*
        }

        #[cfg(feature = "harness")]
        impl Ios {
            /// Keep every half nobody claimed alive in `keep`, so sends to it do not fail.
            pub(crate) fn keep_unclaimed(self, keep: &mut Vec<Box<dyn std::any::Any>>) {
                $(self.$field.keep(keep);)*
            }
        }
    };
}
