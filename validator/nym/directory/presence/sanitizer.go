package presence

import (
	"github.com/microcosm-cc/bluemonday"
	"github.com/nymtech/nym-directory/models"
)

// CocoHostSanitizer cleans untrusted input fields
type CocoHostSanitizer interface {
	Sanitize(models.CocoHostInfo) models.CocoHostInfo
}

// MixHostSanitizer cleans untrusted input fields
type MixHostSanitizer interface {
	Sanitize(models.MixHostInfo) models.MixHostInfo
}

// MixProviderHostSanitizer cleans untrusted input fields
type MixProviderHostSanitizer interface {
	Sanitize(models.MixProviderHostInfo) models.MixProviderHostInfo
}

// GatewayHostSanitizer cleans untrusted input fields
type GatewayHostSanitizer interface {
	Sanitize(models.GatewayHostInfo) models.GatewayHostInfo
}

// IMixNodeIDSanitizer cleans untrusted input fields
type IMixNodeIDSanitizer interface {
	Sanitize(models.MixNodeID) models.MixNodeID
}

// NewCoconodeSanitizer constructor...
func NewCoconodeSanitizer(p *bluemonday.Policy) CoconodeSanitizer {
	return CoconodeSanitizer{
		policy: p,
	}
}

// CoconodeSanitizer kills untrusted input in CocoHostInfo structs
type CoconodeSanitizer struct {
	policy *bluemonday.Policy
}

// Sanitize CocoHostInfo input
func (s *CoconodeSanitizer) Sanitize(input models.CocoHostInfo) models.CocoHostInfo {
	input.PubKey = s.policy.Sanitize(input.PubKey)
	input.Host = s.policy.Sanitize(input.Host)
	input.Type = s.policy.Sanitize(input.Type)
	return input
}

// NewMixnodeSanitizer constructor...
func NewMixnodeSanitizer(p *bluemonday.Policy) MixnodeSanitizer {
	return MixnodeSanitizer{
		policy: p,
	}
}

// MixnodeSanitizer kills untrusted input in MixHostInfo structs
type MixnodeSanitizer struct {
	policy *bluemonday.Policy
}

// Sanitize MixHostInfo input
func (s *MixnodeSanitizer) Sanitize(input models.MixHostInfo) models.MixHostInfo {
	input.PubKey = s.policy.Sanitize(input.PubKey)
	input.Host = s.policy.Sanitize(input.Host)
	return input
}

// NewMixproviderSanitizer constructor...
func NewMixproviderSanitizer(p *bluemonday.Policy) MixproviderSanitizer {
	return MixproviderSanitizer{
		policy: p,
	}
}

// MixproviderSanitizer kills untrusted input in MixProviderHostInfo structs
type MixproviderSanitizer struct {
	policy *bluemonday.Policy
}

// Sanitize MixProviderHostInfo input
func (s *MixproviderSanitizer) Sanitize(input models.MixProviderHostInfo) models.MixProviderHostInfo {
	input.PubKey = s.policy.Sanitize(input.PubKey)
	input.ClientListener = s.policy.Sanitize(input.ClientListener)
	input.MixnetListener = s.policy.Sanitize(input.MixnetListener)
	return input
}

// NewGatewaySanitizer constructor...
func NewGatewaySanitizer(p *bluemonday.Policy) GatewaySanitizer {
	return GatewaySanitizer{
		policy: p,
	}
}

// GatewaySanitizer kills untrusted input in GatewayHostInfo structs
type GatewaySanitizer struct {
	policy *bluemonday.Policy
}

// Sanitize GatewayHostInfo input
func (s *GatewaySanitizer) Sanitize(input models.GatewayHostInfo) models.GatewayHostInfo {
	input.IdentityKey = s.policy.Sanitize(input.IdentityKey)
	input.SphinxKey = s.policy.Sanitize(input.SphinxKey)
	input.ClientListener = s.policy.Sanitize(input.ClientListener)
	input.MixnetListener = s.policy.Sanitize(input.MixnetListener)
	return input
}

// NewMixnodeIDSanitizer ...
func NewMixnodeIDSanitizer(p *bluemonday.Policy) MixNodeIDSanitizer {
	return MixNodeIDSanitizer{
		policy: p,
	}
}

// MixNodeIDSanitizer kills untrusted input in CocoHostInfo structs
type MixNodeIDSanitizer struct {
	policy *bluemonday.Policy
}

// Sanitize MixNodeID input
func (s *MixNodeIDSanitizer) Sanitize(input models.MixNodeID) models.MixNodeID {
	input.PubKey = s.policy.Sanitize(input.PubKey)
	return input
}
