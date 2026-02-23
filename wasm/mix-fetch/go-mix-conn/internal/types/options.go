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
