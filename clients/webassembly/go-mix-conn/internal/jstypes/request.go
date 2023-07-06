// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package jstypes

type Redirect = string
type Mode = string
type CredentialsMode = string
type ResponseTainting = string
type ReferrerPolicy = string
type Referrer = string

const (
	RequestRedirectError  = "error"
	RequestRedirectManual = "manual"
	RequestRedirectFollow = "follow"

	ResponseTaintingBasic            = "basic"
	ResponseTaintingCors             = "cors"
	ResponseTaintingOpaque           = "opaque"
	ResponseTaintingUnsafeIgnoreCors = "ignore-cors"

	ModeCors             = "cors"
	ModeSameOrigin       = "same-origin"
	ModeNoCors           = "no-cors"
	ModeNavigate         = "navigate"
	ModeWebsocket        = "websocket"
	ModeUnsafeIgnoreCors = "unsafe-ignore-cors"

	CredentialsModeOmit       = "omit"
	CredentialsModeSameOrigin = "same-origin"
	CredentialsModeInclude    = "include"

	ReferrerPolicyNoReferrer                  = "no-referrer"
	ReferrerPolicyNoReferrerWhenDowngrade     = "no-referrer-when-downgrade"
	ReferrerPolicyOrigin                      = "origin"
	ReferrerPolicyOriginWhenCrossOrigin       = "origin-when-cross-origin"
	ReferrerPolicySameOrigin                  = "same-origin"
	ReferrerPolicyStrictOrigin                = "strict-origin"
	ReferrerPolicyStrictOriginWhenCrossOrigin = "strict-origin-when-cross-origin"
	ReferrerPolicyUnsafeUrl                   = "unsafe-url"

	ReferrerNoReferrer = "no-referrer"
	ReferrerClient     = "client"
)
