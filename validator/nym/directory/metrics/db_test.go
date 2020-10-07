package metrics

import (
	"github.com/BorisBorshevsky/timemock"
	"github.com/nymtech/nym-directory/models"
	. "github.com/onsi/ginkgo"
	"github.com/stretchr/testify/assert"
)

var _ = Describe("Metrics Db", func() {
	var db *Db
	var metric1 models.MixMetric
	var metric2 models.MixMetric
	var p1 models.PersistedMixMetric
	var p2 models.PersistedMixMetric

	var received uint = 99
	var now = timemock.Now().UnixNano()

	// set up fixtures
	metric1 = models.MixMetric{
		PubKey:   "key1",
		Received: &received,
		Sent:     map[string]uint{"mixnode3": 99, "mixnode4": 100},
	}
	p1 = models.PersistedMixMetric{
		MixMetric: metric1,
		Timestamp: now,
	}

	metric2 = models.MixMetric{
		PubKey:   "key2",
		Received: &received,
		Sent:     map[string]uint{"mixnode3": 101, "mixnode4": 102},
	}
	p2 = models.PersistedMixMetric{
		MixMetric: metric2,
		Timestamp: now,
	}

	Describe("retrieving mixnet metrics", func() {
		Context("when no metrics have been added", func() {
			It("should return an empty metrics list", func() {
				db = NewDb()
				assert.Len(GinkgoT(), db.List(), 0)
			})
		})
	})
	Describe("adding mixnet metrics", func() {
		Context("adding 1", func() {
			It("should contain 1 metric", func() {
				db = NewDb()
				db.Add(p1)
				assert.Len(GinkgoT(), db.List(), 0)          // see note on db.clear()
				assert.Len(GinkgoT(), db.incomingMetrics, 1) // see note on db.clear()
			})
		})
		Context("adding 2", func() {
			It("should contain 2 metrics", func() {
				db = NewDb()
				db.Add(p1)
				db.Add(p2)
				assert.Len(GinkgoT(), db.List(), 0)          // see note on db.clear()
				assert.Len(GinkgoT(), db.incomingMetrics, 2) // see note on db.clear()
			})
		})
	})
})
