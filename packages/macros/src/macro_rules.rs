#[macro_export]
macro_rules! loc_string {
    () => {{
        format!("File: {}:{}\nColumn: {}", file!(), line!(), column!())
    }};
}

#[macro_export]
macro_rules! cosmwasm_imports {
    ($($item:ident),+ $(,)?) => {
        #[cfg(all(feature = "secretwasm",not(feature = "cosmwasm")))]
        use secret_cosmwasm_std::{$($item),+};
        #[cfg(feature = "cosmwasm")]
        use cosmwasm_std::{$($item),+};
    };
}

#[macro_export]
macro_rules! cosmwasm_testing_imports {
    ($($item:ident),+ $(,)?) => {
        #[cfg(all(feature = "secretwasm",not(feature = "cosmwasm")))]
        use secret_cosmwasm_std::testing::{$($item),+};
        #[cfg(feature = "cosmwasm")]
        use cosmwasm_std::testing::{$($item),+};
    };
}

#[macro_export]
macro_rules! adapt_schema {
    ($new_type:ident, $secret_type:ty, $($field:ident : $field_type:ty),*) => {
        pub struct $new_type {
            $(
                pub $field: $field_type,
            )*
        }

        impl From<$secret_type> for $new_type {
            fn from(item: $secret_type) -> Self {
                Self {
                    $(
                        $field: item.$field,
                    )*
                }
            }
        }
    };
}
