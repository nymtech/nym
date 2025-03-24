// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_network_defaults::mainnet::read_var_if_not_default;
use nym_network_defaults::var_names::CONFIGURED;
use std::any::type_name;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

pub const MISSING_VALUE: &str = "MISSING VALUE";

/// Helper for providing default value for templated config fields.
pub fn missing_string_value<T: From<String>>() -> T {
    MISSING_VALUE.to_string().into()
}

/// Helper for providing default INADDR_ANY IpAddr, i.e. `0.0.0.0`
pub fn inaddr_any() -> IpAddr {
    IpAddr::V4(Ipv4Addr::UNSPECIFIED)
}

/// Helper for providing default IN6ADDR_ANY_INIT IpAddr, i.e. `::`
pub fn in6addr_any_init() -> IpAddr {
    IpAddr::V6(Ipv6Addr::UNSPECIFIED)
}

// TODO: is it really part of 'Config'?
pub trait OptionalSet {
    /// If the value is available (i.e. `Some`), the provided closure is applied.
    /// Otherwise `self` is returned with no modifications.
    fn with_optional<F, T>(self, f: F, val: Option<T>) -> Self
    where
        F: Fn(Self, T) -> Self,
        Self: Sized,
    {
        if let Some(val) = val {
            f(self, val)
        } else {
            self
        }
    }

    /// If the value is available (i.e. `Some`) it is validated and then the provided closure is applied.
    /// Otherwise `self` is returned with no modifications.
    fn with_validated_optional<F, T, V, E>(
        self,
        f: F,
        value: Option<T>,
        validate: V,
    ) -> Result<Self, E>
    where
        F: Fn(Self, T) -> Self,
        V: Fn(&T) -> Result<(), E>,
        Self: Sized,
    {
        if let Some(val) = value {
            validate(&val)?;
            Ok(f(self, val))
        } else {
            Ok(self)
        }
    }

    /// If the value is available (i.e. `Some`), the provided closure is applied.
    /// Otherwise, if the environment was configured and the corresponding variable was set,
    /// the value is parsed using the `FromStr` implementation and the closure is applied on that instead.
    /// Finally, if none of those were available, `self` is returned with no modifications.
    fn with_optional_env<F, T>(self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(Self, T) -> Self,
        T: FromStr,
        <T as FromStr>::Err: Debug,
        Self: Sized,
    {
        if let Some(val) = val {
            return f(self, val);
        } else if std::env::var(CONFIGURED).is_ok() {
            if let Some(raw) = read_var_if_not_default(env_var) {
                return f(
                    self,
                    raw.parse().unwrap_or_else(|err| {
                        panic!(
                            "failed to parse value of {raw} into type {}. the error was {:?}",
                            type_name::<T>(),
                            err
                        )
                    }),
                );
            }
        }
        self
    }

    /// If the value is available (i.e. `Some`), the provided closure is applied.
    /// Otherwise, if the environment was configured and the corresponding variable was set,
    /// the value is parsed using the provided parser and the closure is applied on that instead.
    /// Finally, if none of those were available, `self` is returned with no modifications.
    fn with_optional_custom_env<F, T, G>(
        self,
        f: F,
        val: Option<T>,
        env_var: &str,
        parser: G,
    ) -> Self
    where
        F: Fn(Self, T) -> Self,
        G: Fn(&str) -> T,
        Self: Sized,
    {
        if let Some(val) = val {
            return f(self, val);
        } else if std::env::var(CONFIGURED).is_ok() {
            if let Some(raw) = read_var_if_not_default(env_var) {
                return f(self, parser(&raw));
            }
        }
        self
    }
}

// helper for when we want to use `OptionalSet` on an inner field
// (used by clients wanting to set the `BaseConfig` values)
#[macro_export]
macro_rules! define_optional_set_inner {
    ( $x: ident, $inner_field_name: ident, $inner_field_typ: ty ) => {
        impl $x {
            pub fn with_optional_inner<F, T>(mut self, f: F, val: Option<T>) -> Self
            where
                F: Fn($inner_field_typ, T) -> $inner_field_typ,
            {
                self.$inner_field_name = self.$inner_field_name.with_optional(f, val);
                self
            }

            pub fn with_validated_optional_inner<F, T, V, E>(
                mut self,
                f: F,
                value: Option<T>,
                validate: V,
            ) -> Result<Self, E>
            where
                F: Fn($inner_field_typ, T) -> $inner_field_typ,
                V: Fn(&T) -> Result<(), E>,
            {
                self.$inner_field_name = self
                    .$inner_field_name
                    .with_validated_optional(f, value, validate)?;
                Ok(self)
            }

            pub fn with_optional_env_inner<F, T>(
                mut self,
                f: F,
                val: Option<T>,
                env_var: &str,
            ) -> Self
            where
                F: Fn($inner_field_typ, T) -> $inner_field_typ,
                T: FromStr,
                <T as FromStr>::Err: Debug,
            {
                self.$inner_field_name = self.$inner_field_name.with_optional_env(f, val, env_var);
                self
            }

            pub fn with_optional_custom_env_inner<F, T, G>(
                mut self,
                f: F,
                val: Option<T>,
                env_var: &str,
                parser: G,
            ) -> Self
            where
                F: Fn($inner_field_typ, T) -> $inner_field_typ,
                G: Fn(&str) -> T,
            {
                self.$inner_field_name = self
                    .$inner_field_name
                    .with_optional_custom_env(f, val, env_var, parser);
                self
            }
        }
    };
}

// this function is only used for parsing values from the network defaults and thus the "expect" there are fine
pub fn parse_urls(raw: &str) -> Vec<url::Url> {
    raw.split(',')
        .map(|raw_url| {
            raw_url
                .trim()
                .parse()
                .expect("one of the provided urls was invalid")
        })
        .collect()
}

impl<T> OptionalSet for T {}
