#[macro_export]
macro_rules! type_list {
    () => { $crate::type_list::Nil };
    ($head:ty, $($tail:ty,)*) => { $crate::type_list::Cons<$head, $crate::type_list!($($tail,)*)> };
}

#[macro_export]
macro_rules! derive_dependencies_from {
    (
        pub struct $name:ident<'context> {
            $(
                $dependency_field:ident : $dependency:ty,
            )*
        }
    ) => {
        pub struct $name<'context> {
            $($dependency_field: $dependency,)*
        }

        impl<'context, ContextT, IndicesT>
            $crate::context::DependenciesFrom<ContextT, IndicesT>
            for $name<'context>
        where
            ContextT: $crate::type_list::PluckList<
                $crate::type_list!($($dependency,)*),
                IndicesT,
            >,
        {
            fn dependencies_from(context: ContextT) -> Self {
                let (rest, _) = context.pluck_list();
                $(
                    let $crate::type_list::Cons { head: $dependency_field, tail: rest } = rest;
                )*
                let _ = rest;
                $name { $($dependency_field,)* }
            }
        }
    };
}
