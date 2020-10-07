package fixtures

import "github.com/nymtech/nym-directory/models"

func MixMetricsList() []models.PersistedMixMetric {
	r1 := uint(1)
	m1 := models.PersistedMixMetric{
		MixMetric: models.MixMetric{
			PubKey:   "pubkey1",
			Received: &r1,
		},
	}

	r2 := uint(2)
	m2 := models.PersistedMixMetric{
		MixMetric: models.MixMetric{
			PubKey:   "pubkey2",
			Received: &r2,
		},
	}

	metrics := []models.PersistedMixMetric{m1, m2}
	return metrics
}
