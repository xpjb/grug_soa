/// Trait for types that can append entity/component data from a JSON "prototype".
///
/// The expected input is a JSON object whose keys match ECS storage field names.
pub trait LoadPrototype {
    fn load_prototype(&mut self, prototype: serde_json::Value);
}

/// Implements [`LoadPrototype`] for a "struct-of-Vecs" ECS by mapping JSON keys to storages.
///
/// This is a `macro_rules!` stand-in for a desired `#[derive(LoadPrototype)]` API (which would
/// require a proc-macro to introspect struct fields).
///
/// Example:
///
/// ```ignore
/// #[derive(Default)]
/// struct MyECS { foo: Vec<Foo>, bar: Vec<Bar> }
///
/// grug_soa::impl_load_prototype!(MyECS { foo: Foo, bar: Bar });
/// ```
#[macro_export]
macro_rules! impl_load_prototype {
    ($ecs:ty { $($field:ident : $ty:ty),* $(,)? }) => {
        impl $ecs {
            /// Copies (clones) one "entity" worth of registered component fields from `prototype`
            /// into `self`, chosen by `prototype_index`.
            ///
            /// This is intended for runtime spawning, where no deserialization happens.
            pub fn spawn_entity(&mut self, prototype: &$ecs, prototype_index: usize)
            where
                $(
                    $ty: ::core::clone::Clone,
                )*
            {
                $(
                    let v = prototype.$field
                        .get(prototype_index)
                        .unwrap_or_else(|| {
                            panic!(
                                concat!(
                                    "prototype_index out of bounds for field `",
                                    stringify!($field),
                                    "`"
                                )
                            )
                        })
                        .clone();
                    self.$field.push(v);
                )*
            }
        }

        impl $crate::LoadPrototype for $ecs
        where
            $(
                $ty: ::core::default::Default + ::core::clone::Clone + ::serde::de::DeserializeOwned,
            )*
        {
            fn load_prototype(&mut self, prototype: ::serde_json::Value) {
                let obj = prototype
                    .as_object()
                    .expect("prototype must be a JSON object");

                $(
                    if let Some(v) = obj.get(::core::stringify!($field)) {
                        self.$field.push(
                            ::serde_json::from_value::<$ty>(v.clone())
                                .expect(::core::concat!("invalid ", ::core::stringify!($field))),
                        );
                    } else {
                        self.$field.push(<$ty as ::core::default::Default>::default());
                    }
                )*
            }
        }
    };
}


