// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! wasm_wrapper {
    ($base:ident, $wrapper:ident) => {
        #[wasm_bindgen]
        pub struct $wrapper {
            pub(crate) inner: $base,
        }

        impl std::ops::Deref for $wrapper {
            type Target = $base;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl From<$base> for $wrapper {
            fn from(inner: $base) -> Self {
                $wrapper { inner }
            }
        }

        impl From<$wrapper> for $base {
            fn from(value: $wrapper) -> Self {
                value.inner
            }
        }
    };
}

#[macro_export]
macro_rules! data_pointer_clone {
    ($wrapper:ident) => {
        #[wasm_bindgen]
        impl $wrapper {
            #[wasm_bindgen(js_name = "cloneDataPointer")]
            pub fn clone_data_pointer(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! wasm_wrapper_bs58 {
    ($base:ident, $wrapper:ident) => {
        wasm_wrapper!($base, $wrapper);

        impl std::fmt::Display for $wrapper {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.inner.to_bs58().fmt(f)
            }
        }

        impl FromStr for $wrapper {
            type Err = ZkNymError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok($base::try_from_bs58(s)?.into())
            }
        }

        #[wasm_bindgen]
        impl $wrapper {
            pub fn stringify(&self) -> String {
                self.to_string()
            }

            #[wasm_bindgen(js_name = "fromString")]
            pub fn from_string(raw: String) -> Result<$wrapper, ZkNymError> {
                raw.parse()
            }
        }
    };
}
