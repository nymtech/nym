// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// A simple macro that given `TryFrom<&A> for B`, implements `TryFrom<A> for B`
/// using the former implementation
#[macro_export]
macro_rules! impl_tryfrom_ref {
    ($src:ty, $dst:ty, $err:ty) => {
        impl TryFrom<$src> for $dst {
            // can't use type Error = <Self as TryFrom<&$src>>::Error;
            // due to lifetime interference within macros
            type Error = $err;

            fn try_from(value: $src) -> Result<Self, Self::Error> {
                <Self as TryFrom<&$src>>::try_from(&value)
            }
        }
    };
}

/// A simple macro that implements all required variants of `TryFrom`
/// between particular versioned `VersionedRequest` and given request variant
/// using default bincode serializer
#[macro_export]
macro_rules! impl_default_bincode_request_query_conversions {
    // limitation of macros - need to pass the same underlying type twice,
    // once as pattern and once as expression
    ($top_req_type:ty, $inner_req_type:ty, $query_type_pat:pat, $query_type_expr:expr) => {
        $crate::impl_query_conversions!(
            $crate::Request,
            $top_req_type,
            $inner_req_type,
            $query_type_pat,
            $query_type_expr
        );
    };
}

/// A simple macro that implements all required variants of `TryFrom`
/// between particular versioned `VersionedResponse` and given response variant
/// using default bincode serializer
#[macro_export]
macro_rules! impl_default_bincode_response_query_conversions {
    // limitation of macros - need to pass the same underlying type twice,
    // once as pattern and once as expression
    ($top_resp_type:ty, $inner_resp_type:ty, $query_type_pat:pat, $query_type_expr:expr) => {
        $crate::impl_query_conversions!(
            $crate::Response,
            $top_resp_type,
            $inner_resp_type,
            $query_type_pat,
            $query_type_expr
        );
    };
}

/// A simple macro that implements all required variants of `TryFrom`
/// between [crate::models::Request] and corresponding versioned `VersionedRequest`
/// using default bincode serializer
#[macro_export]
macro_rules! impl_default_bincode_request_conversions {
    ($req_type:ty, $version:expr) => {
        $crate::impl_versioned_conversions!($crate::Request, $req_type, $version);
    };
}

/// A simple macro that implements all required variants of `TryFrom`
/// between [crate::models::Response] and corresponding versioned `VersionedResponse`
/// using default bincode serializer
#[macro_export]
macro_rules! impl_default_bincode_response_conversions {
    ($req_type:ty, $version:expr) => {
        $crate::impl_versioned_conversions!($crate::Response, $req_type, $version);
    };
}

#[macro_export]
macro_rules! impl_versioned_conversions {
    (
        // is it Request or Response
        $main_type_ty:ty,

        // e.g. VersionedResponse
        $top_type:ty,

        // request/response version type
        $version:expr
    ) => {
        impl TryFrom<&$top_type> for $main_type_ty {
            type Error = $crate::models::error::Error;

            fn try_from(value: &$top_type) -> Result<Self, Self::Error> {
                use ::bincode::Options;
                let data = $crate::make_bincode_serializer().serialize(value)?;
                Ok(<$main_type_ty>::new($version, data))
            }
        }

        // automatically generate `impl TryFrom<$top_type> for $main_type`
        $crate::impl_tryfrom_ref!($top_type, $main_type_ty, $crate::models::error::Error);

        impl TryFrom<&$main_type_ty> for $top_type {
            type Error = $crate::models::error::Error;

            fn try_from(value: &$main_type_ty) -> Result<Self, Self::Error> {
                use ::bincode::Options;
                if value.version != $version {
                    return Err($crate::models::error::Error::InvalidVersion {
                        source_version: value.version,
                        target_version: $version,
                    });
                }
                Ok($crate::make_bincode_serializer().deserialize(&value.inner)?)
            }
        }

        // automatically generate `impl TryFrom<$main_type> for $top_type`
        $crate::impl_tryfrom_ref!($main_type_ty, $top_type, $crate::models::error::Error);
    };
}

#[macro_export]
macro_rules! impl_query_conversions {
    // limitation of macros - need to pass the same underlying type twice,
    // once as pattern and once as expression
    (
        // is it Request or Response
        $main_type:ty,

        // e.g. VersionedResponse
        $top_type:ty,

        // e.g. InnerTopUpResponse
        $inner_type:ty,

        // e.g. QueryType::TopUpBandwidth,
        $query_type_pat:pat,

        // e.g. QueryType::TopUpBandwidth,
        $query_type_expr:expr
    ) => {
        // conversion from the versioned type into the particular typ,
        // e.g. TryFrom<&VersionedResponse> for InnerTopUpResponse
        impl TryFrom<&$top_type> for $inner_type {
            type Error = $crate::models::error::Error;

            fn try_from(value: &$top_type) -> Result<Self, Self::Error> {
                use ::bincode::Options;
                match value.query_type {
                    $query_type_pat => {
                        Ok($crate::make_bincode_serializer().deserialize(&value.inner)?)
                    }
                    other => Err($crate::models::error::Error::InvalidQueryType {
                        source_query_type: other.to_string(),
                        target_query_type: stringify!($query_type_pat).to_string(),
                    }),
                }
            }
        }
        // implementation of conversion without the referenced type, i.e.
        // e.g. TryFrom<VersionedResponse> for InnerTopUpResponse
        $crate::impl_tryfrom_ref!($top_type, $inner_type, $crate::models::error::Error);

        // conversion back from the particular type into the versioned type, i.e.
        // e.g. TryFrom<&InnerTopUpResponse> for VersionedResponse
        impl TryFrom<&$inner_type> for $top_type {
            type Error = $crate::models::error::Error;

            fn try_from(value: &$inner_type) -> Result<Self, Self::Error> {
                use ::bincode::Options;
                Ok(Self {
                    query_type: $query_type_expr,
                    inner: $crate::make_bincode_serializer().serialize(value)?,
                })
            }
        }

        // implementation of conversion without the referenced type, i.e.
        // e.g. TryFrom<InnerTopUpResponse> for VersionedResponse
        $crate::impl_tryfrom_ref!($inner_type, $top_type, $crate::models::error::Error);

        // conversion from the'main' type (Request/Response) into the particular type
        // e.g. TryFrom<&Response> for InnerTopUpResponse
        impl TryFrom<&$main_type> for $inner_type {
            type Error = $crate::error::MetadataError;

            fn try_from(value: &$main_type) -> Result<Self, Self::Error> {
                <$top_type>::try_from(value)?.try_into().map_err(
                    |err: $crate::models::error::Error| $crate::error::MetadataError::Models {
                        message: err.to_string(),
                    },
                )
            }
        }

        // implementation of conversion without the referenced type, i.e.
        // e.g. TryFrom<Response> for InnerTopUpResponse
        $crate::impl_tryfrom_ref!($main_type, $inner_type, $crate::error::MetadataError);

        // conversion from the particular type into the 'main' type (Request/Response)
        // e.g. TryFrom<&InnerTopUpResponse> for Response
        impl TryFrom<&$inner_type> for $main_type {
            type Error = $crate::error::MetadataError;

            fn try_from(value: &$inner_type) -> Result<Self, Self::Error> {
                <$top_type>::try_from(value)?.try_into().map_err(
                    |err: $crate::models::error::Error| $crate::error::MetadataError::Models {
                        message: err.to_string(),
                    },
                )
            }
        }

        // implementation of conversion without the referenced type, i.e.
        // e.g. TryFrom<InnerTopUpResponse> for Response
        $crate::impl_tryfrom_ref!($inner_type, $main_type, $crate::error::MetadataError);
    };
}
