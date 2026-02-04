// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package types

import (
	"fmt"
	"go-mix-conn/internal/jstypes"
)

type RequestOptions struct {
	Redirect         jstypes.Redirect
	Mode             jstypes.Mode
	CredentialsMode  jstypes.CredentialsMode
	ReferrerPolicy   jstypes.ReferrerPolicy
	Referrer         jstypes.Referrer
	ResponseTainting jstypes.ResponseTainting
	Method           string

	// RequestURL stores the full URL (inc path and query params) for this request.
	// Used as the key for request deduplication, allowing concurrent requests to different
	// paths on the same domain (e.g., foo.com/index.html and foo.com/index.js can run
	// concurrently, but two requests to the exact same URL will be blocked).
	RequestURL string
}

func (opts RequestOptions) String() string {
	return fmt.Sprintf(
		"{ Redirect: %s, Mode: %s, credentials: %s, ReferrerPolicy: %s, Referrer: %s, ResponseTainting: %s, Method: %s }",
		opts.Redirect,
		opts.Mode,
		opts.CredentialsMode,
		opts.ReferrerPolicy,
		opts.Referrer,
		opts.ResponseTainting,
		opts.Method,
	)
}

type RequestContext struct {
	WasRedirected           bool
	OverwrittenResponseType jstypes.ResponseType
}
