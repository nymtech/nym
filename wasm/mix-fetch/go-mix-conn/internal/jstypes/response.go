// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package jstypes

type ResponseType = string

const (
	ResponseTypeBasic            = "basic"
	ResponseTypeCors             = "cors"
	ResponseTypeDefault          = "default"
	ResponseTypeError            = "error"
	ResponseTypeOpaque           = "opaque"
	ResponseTypeOpaqueRedirect   = "opaqueredirect"
	ResponseTypeUnsafeIgnoreCors = "unsafe-ignore-cors"
)
