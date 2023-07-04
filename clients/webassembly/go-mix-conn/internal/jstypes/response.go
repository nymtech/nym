// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package jstypes

type ResponseType = string

const (
	RESPONSE_TYPE_BASIC              = "basic"
	RESPONSE_TYPE_CORS               = "cors"
	RESPONSE_TYPE_DEFAULT            = "default"
	RESPONSE_TYPE_ERROR              = "error"
	RESPONSE_TYPE_OPAQUE             = "opaque"
	RESPONSE_TYPE_OPAQUE_REDIRECT    = "opaqueredirect"
	RESPONSE_TYPE_UNSAFE_IGNORE_CORS = "unsafe-ignore-cors"
)
