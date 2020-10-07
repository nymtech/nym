package metrics

import (
	"encoding/json"

	"github.com/BorisBorshevsky/timemock"
	"github.com/nymtech/nym-directory/metrics/mocks"
	"github.com/nymtech/nym-directory/models"
	. "github.com/onsi/ginkgo"
	"gotest.tools/assert"

	wsMocks "github.com/nymtech/nym-directory/server/websocket/mocks"
)

var _ = Describe("metrics.Service", func() {
	var mockDb mocks.IDb
	var m1 models.MixMetric
	var m2 models.MixMetric
	var p1 models.PersistedMixMetric
	var p2 models.PersistedMixMetric

	var serv Service
	var received uint = 99
	var now = timemock.Now()
	timemock.Freeze(now)
	var frozenNow = timemock.Now().UnixNano()

	// set up fixtures
	m1 = models.MixMetric{
		PubKey:   "key1",
		Received: &received,
		Sent:     map[string]uint{"mixnode3": 99, "mixnode4": 101},
	}

	p1 = models.PersistedMixMetric{
		MixMetric: m1,
		Timestamp: frozenNow,
	}

	m2 = models.MixMetric{
		PubKey:   "key2",
		Received: &received,
		Sent:     map[string]uint{"mixnode3": 102, "mixnode4": 103},
	}

	p2 = models.PersistedMixMetric{
		MixMetric: m2,
		Timestamp: frozenNow,
	}

	Describe("Adding a mixmetric", func() {
		It("should add a PersistedMixMetric to the db and notify the Hub", func() {
			mockDb = *new(mocks.IDb)
			mockHub := *new(wsMocks.IHub)
			serv = *NewService(&mockDb, &mockHub)
			mockDb.On("Add", p1)
			j, _ := json.Marshal(p1)
			mockHub.On("Notify", j)

			serv.CreateMixMetric(m1)

			mockDb.AssertCalled(GinkgoT(), "Add", p1)
			mockHub.AssertCalled(GinkgoT(), "Notify", j)
		})
	})
	Describe("Listing mixmetrics", func() {
		Context("when receiving a list request", func() {
			It("should call to the Db", func() {
				mockDb = *new(mocks.IDb)
				mockHub := *new(wsMocks.IHub)

				list := []models.PersistedMixMetric{p1, p2}

				serv = *NewService(&mockDb, &mockHub)
				mockDb.On("List").Return(list)

				result := serv.List()

				mockDb.AssertCalled(GinkgoT(), "List")
				assert.Equal(GinkgoT(), list[0].MixMetric.PubKey, result[0].MixMetric.PubKey)
				assert.Equal(GinkgoT(), list[1].MixMetric.PubKey, result[1].MixMetric.PubKey)
			})
		})
	})
})
