package mixmining

import (
	"github.com/microcosm-cc/bluemonday"
	"github.com/nymtech/nym-directory/models"
)

// Sanitizer sanitizes untrusted metrics data. It should be used in
// controllers to wipe out any questionable input at our application's front
// door.
type Sanitizer interface {
	Sanitize(input models.MixStatus) models.MixStatus
}

type sanitizer struct {
	policy *bluemonday.Policy
}

// NewSanitizer returns a new input sanitizer for metrics
func NewSanitizer(policy *bluemonday.Policy) Sanitizer {
	return sanitizer{
		policy: policy,
	}
}

func (s sanitizer) Sanitize(input models.MixStatus) models.MixStatus {
	sanitized := newMeasurement()

	sanitized.PubKey = s.policy.Sanitize(input.PubKey)
	sanitized.IPVersion = s.policy.Sanitize(input.IPVersion)
	sanitized.Up = input.Up
	return sanitized
}

func newMeasurement() models.MixStatus {
	booltrue := true
	return models.MixStatus{
		PubKey:    "",
		IPVersion: "",
		Up:        &booltrue,
	}
}
