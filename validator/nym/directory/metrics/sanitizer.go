package metrics

import (
	"github.com/microcosm-cc/bluemonday"
	"github.com/nymtech/nym-directory/models"
)

// Sanitizer sanitizes untrusted metrics data. It should be used in
// controllers to wipe out any questionable input at our application's front
// door.
type Sanitizer interface {
	Sanitize(input models.MixMetric) models.MixMetric
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

func (s sanitizer) Sanitize(input models.MixMetric) models.MixMetric {
	sanitized := newMetric()

	sanitized.PubKey = s.policy.Sanitize(input.PubKey)
	for key, value := range input.Sent {
		k := bluemonday.UGCPolicy().Sanitize(key)
		sanitized.Sent[k] = value
	}
	sanitized.Received = input.Received
	return sanitized
}

func newMetric() models.MixMetric {
	sent := make(map[string]uint)
	received := uint(1)
	return models.MixMetric{
		PubKey:   "",
		Sent:     sent,
		Received: &received,
	}
}
