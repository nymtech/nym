package presence

import (
	"github.com/BorisBorshevsky/timemock"
	"github.com/nymtech/nym-directory/models"
	"github.com/nymtech/nym-directory/presence/mocks"
	. "github.com/onsi/ginkgo"
	_ "github.com/onsi/gomega"
	"github.com/stretchr/testify/assert"
)

var _ = Describe("presence.Service", func() {
	var (
		mix1              models.MixHostInfo
		mix2              models.MixHostInfo
		mix3              models.MixHostInfo
		mixpresence1      models.MixNodePresence
		mixpresence2      models.MixNodePresence
		mixpresence3      models.MixNodePresence
		coco1             models.CocoHostInfo
		cocopresence2     models.CocoPresence
		provider1         models.MixProviderHostInfo
		providerpresence3 models.MixProviderPresence
		mockDb            mocks.IDb

		serv Service
	)
	BeforeEach(func() {
		mockDb = *new(mocks.IDb)
		serv = *NewService(&mockDb)
		var now = timemock.Now()
		timemock.Freeze(now)

		// Set up fixtures
		mix1 = models.MixHostInfo{
			HostInfo: models.HostInfo{
				Host:     "foo.com:8000",
				PubKey:   "pubkey1",
				Location: defaultLocation,
			},
			Layer: 1,
		}

		mixpresence1 = models.MixNodePresence{
			MixHostInfo: mix1,
			LastSeen:    timemock.Now().UnixNano(),
		}

		mix2 = models.MixHostInfo{
			HostInfo: models.HostInfo{
				Host:     "floop.com:8000",
				PubKey:   "pubkey2",
				Location: defaultLocation,
			},
			Layer: 1,
		}

		mixpresence2 = models.MixNodePresence{
			MixHostInfo: mix2,
			LastSeen:    timemock.Now().UnixNano(),
		}

		mix3 = models.MixHostInfo{
			HostInfo: models.HostInfo{
				Host:     "snoop.com:8000",
				PubKey:   "pubkey3",
				Location: defaultLocation,
			},
			Layer: 1,
		}

		mixpresence3 = models.MixNodePresence{
			MixHostInfo: mix3,
			LastSeen:    timemock.Now().UnixNano(),
		}

		coco1 = models.CocoHostInfo{
			HostInfo: models.HostInfo{
				Host:     "bar.com:8000",
				PubKey:   "pubkey2",
				Location: defaultLocation,
			},
			Type: "foo",
		}
		cocopresence2 = models.CocoPresence{
			CocoHostInfo: coco1,
			LastSeen:     timemock.Now().UnixNano(),
		}

		provider1 = models.MixProviderHostInfo{
			MixnetListener: "foo.com:8000",
			ClientListener: "foo.com:8001",
			Location:       defaultLocation,
			PubKey:         "pubkey2",
		}

		providerpresence3 = models.MixProviderPresence{
			MixProviderHostInfo: provider1,
			LastSeen:            timemock.Now().UnixNano(),
		}
	})

	Describe("Adding presence info", func() {
		Context("for a mixnode", func() {
			It("should add a presence to the db", func() {
				mockDb.On("AddMix", mixpresence1)
				serv.AddMixNodePresence(mix1)
				mockDb.AssertCalled(GinkgoT(), "AddMix", mixpresence1)
			})
		})
		Context("for a coconode", func() {
			It("should add a presence to the db", func() {
				mockDb.On("AddCoco", cocopresence2)
				serv.AddCocoNodePresence(coco1, "bar.com")
				mockDb.AssertCalled(GinkgoT(), "AddCoco", cocopresence2)
			})
		})
		Context("for a provider node", func() {
			It("should add a presence to the db", func() {
				mockDb.On("AddMixProvider", providerpresence3)
				serv.AddMixProviderPresence(provider1)
				mockDb.AssertCalled(GinkgoT(), "AddMixProvider", providerpresence3)
			})
		})
	})
	Describe("Getting the Topology", func() {
		Context("when receiving a list request", func() {
			It("should call to the Db", func() {
				list := []models.MixNodePresence{
					mixpresence1,
				}
				topology := models.Topology{
					MixNodes: list,
				}
				mockDb.On("Topology").Return(topology)
				mockDb.On("ListDisallowed").Return(make([]string, 0))
				result := serv.Topology()
				mockDb.AssertCalled(GinkgoT(), "Topology")
				mockDb.AssertCalled(GinkgoT(), "ListDisallowed")
				assert.Equal(GinkgoT(), topology, result)
			})
		})

		Context("when there is 1 disallowed nodes", func() {
			It("should remove 1 disallowed mixnodes and return the rest", func() {
				mixnodes := []models.MixNodePresence{
					mixpresence1, mixpresence2,
				}

				dbTopology := models.Topology{
					MixNodes: mixnodes,
				}

				disallowed := []string{mix2.PubKey}

				mockDb.On("Topology").Return(dbTopology)
				mockDb.On("ListDisallowed").Return(disallowed)

				// Now we set up an expectation that mixpresence2 should be in
				// the topology's returned disallowed nodes, but not in the
				// regular mixnodes list
				expectedTopology := models.Topology{
					MixNodes: []models.MixNodePresence{mixpresence1},
				}

				result := serv.Topology()

				mockDb.AssertCalled(GinkgoT(), "Topology")
				mockDb.AssertCalled(GinkgoT(), "ListDisallowed")
				assert.Equal(GinkgoT(), expectedTopology, result)
				assert.NotContains(GinkgoT(), result.MixNodes, mixpresence2)
				assert.Contains(GinkgoT(), result.MixNodes, mixpresence1)
			})
		})

		Context("when there is 1 disallowed node with a base64 pubkey", func() {
			It("should remove 1 disallowed mixnode and return the rest", func() {
				mixpresence2.PubKey = "bzWdTz9E-VD9UWnvDSz5-qEs_lOQ_7PA7cOp9wIwzxI="
				mixnodes := []models.MixNodePresence{
					mixpresence1, mixpresence2,
				}

				dbTopology := models.Topology{
					MixNodes: mixnodes,
				}

				disallowed := []string{mixpresence2.PubKey}

				mockDb.On("Topology").Return(dbTopology)
				mockDb.On("ListDisallowed").Return(disallowed)

				// Now we set up an expectation that mixpresence2 should be in
				// the topology's returned disallowed nodes, but not in the
				// regular mixnodes list
				expectedTopology := models.Topology{
					MixNodes: []models.MixNodePresence{mixpresence1},
				}

				result := serv.Topology()

				mockDb.AssertCalled(GinkgoT(), "Topology")
				mockDb.AssertCalled(GinkgoT(), "ListDisallowed")
				assert.Equal(GinkgoT(), expectedTopology, result)
				assert.NotContains(GinkgoT(), result.MixNodes, mixpresence2)
				assert.Contains(GinkgoT(), result.MixNodes, mixpresence1)
			})
		})

		Context("when there are 2 disallowed nodes out of 3", func() {
			It("should remove 2 disallowed mixnodes and return the other one", func() {
				mixnodes := []models.MixNodePresence{
					mixpresence1, mixpresence2, mixpresence3,
				}

				dbTopology := models.Topology{
					MixNodes: mixnodes,
				}

				disallowed := []string{mix1.PubKey, mix2.PubKey}

				mockDb.On("Topology").Return(dbTopology)
				mockDb.On("ListDisallowed").Return(disallowed)

				// expectedTopologyResult := models.Topology{
				// 	MixNodes: []models.MixNodePresence{mixpresence3},
				// }

				result := serv.Topology()

				mockDb.AssertCalled(GinkgoT(), "Topology")
				mockDb.AssertCalled(GinkgoT(), "ListDisallowed")
				// assert.Equal(GinkgoT(), expectedTopologyResult, result)
				assert.Contains(GinkgoT(), result.MixNodes, mixpresence3)
				assert.NotContains(GinkgoT(), result.MixNodes, mixpresence1)
				assert.NotContains(GinkgoT(), result.MixNodes, mixpresence2)
				assert.Contains(GinkgoT(), result.MixNodes, mixpresence3)
			})
		})
	})

	Describe("Allowing nodes", func() {
		Context("happy path", func() {
			It("should ask the db to allow the provided pubkey", func() {
				node := models.MixNodeID{PubKey: "abc123"}
				mockDb.On("Allow", node.PubKey)
				serv.Allow(node)
				mockDb.AssertCalled(GinkgoT(), "Allow", node.PubKey)
			})
		})
	})
	Describe("Disallowing nodes", func() {
		Context("once", func() {
			It("should ask the db to disallow the provided pubkey", func() {
				node := models.MixNodeID{PubKey: "abc123"}
				mockDb.On("Disallow", node.PubKey)
				serv.Disallow(node)
				mockDb.AssertCalled(GinkgoT(), "Disallow", node.PubKey)
			})
		})
		Context("once with base64 pubkey", func() {
			It("should ask the db to disallow the provided pubkey", func() {
				node := models.MixNodeID{PubKey: "bzWdTz9E-VD9UWnvDSz5-qEs_lOQ_7PA7cOp9wIwzxI="}
				mockDb.On("Disallow", node.PubKey)
				serv.Disallow(node)
				mockDb.AssertCalled(GinkgoT(), "Disallow", node.PubKey)
			})
		})

		Context("twice for the same node", func() {
			It("should ask the db to disallow the provided pubkey twice", func() {
				node := models.MixNodeID{PubKey: "abc123"}
				mockDb.On("Disallow", node.PubKey)
				serv.Disallow(node)
				serv.Disallow(node)
				mockDb.AssertCalled(GinkgoT(), "Disallow", node.PubKey)
				mockDb.AssertNumberOfCalls(GinkgoT(), "Disallow", 2)
			})
		})

		Context("for two different nodes", func() {
			It("should ask the db to disallow the provided pubkeys", func() {
				node1 := models.MixNodeID{PubKey: "abc123"}
				node2 := models.MixNodeID{PubKey: "def456"}
				mockDb.On("Disallow", node1.PubKey)
				mockDb.On("Disallow", node2.PubKey)
				serv.Disallow(node1)
				serv.Disallow(node2)
				mockDb.AssertCalled(GinkgoT(), "Disallow", node1.PubKey)
				mockDb.AssertNumberOfCalls(GinkgoT(), "Disallow", 2)
			})
		})
	})

	Describe("Listing disallowed nodes", func() {
		Context("with 1 disallowed node", func() {
			It("should return a list containing 1 disallowed MixNodePresence objects", func() {
				topology := models.Topology{
					MixNodes: []models.MixNodePresence{mixpresence1, mixpresence2},
				}
				mockDb.On("ListDisallowed").Return([]string{"pubkey1"})
				mockDb.On("Topology").Return(topology)

				expectedDisallowed := []models.MixNodePresence{mixpresence1}

				response := serv.ListDisallowed()
				mockDb.AssertCalled(GinkgoT(), "ListDisallowed")
				mockDb.AssertCalled(GinkgoT(), "Topology")
				assert.Equal(GinkgoT(), expectedDisallowed, response)
			})
		})

		Context("with 1 disallowed node having a base64 pubkey", func() {
			It("should return a list containing 1 disallowed MixNodePresence objects", func() {
				mixpresence1.PubKey = "bzWdTz9E-VD9UWnvDSz5-qEs_lOQ_7PA7cOp9wIwzxI="
				topology := models.Topology{
					MixNodes: []models.MixNodePresence{mixpresence1, mixpresence2},
				}
				mockDb.On("ListDisallowed").Return([]string{mixpresence1.PubKey})
				mockDb.On("Topology").Return(topology)

				expectedDisallowed := []models.MixNodePresence{mixpresence1}

				response := serv.ListDisallowed()
				mockDb.AssertCalled(GinkgoT(), "ListDisallowed")
				mockDb.AssertCalled(GinkgoT(), "Topology")
				assert.Equal(GinkgoT(), expectedDisallowed, response)
			})
		})

		Context("with 2 disallowed nodes", func() {
			It("should return a list containing 2 disallowed MixNodePresence objects", func() {
				topology := models.Topology{
					MixNodes: []models.MixNodePresence{mixpresence1, mixpresence2},
				}
				mockDb.On("ListDisallowed").Return([]string{"pubkey1", "pubkey2"})
				mockDb.On("Topology").Return(topology)

				expectedDisallowed := []models.MixNodePresence{mixpresence1, mixpresence2}

				response := serv.ListDisallowed()
				mockDb.AssertCalled(GinkgoT(), "ListDisallowed")
				mockDb.AssertCalled(GinkgoT(), "Topology")
				assert.Equal(GinkgoT(), expectedDisallowed, response)
			})
		})

		Context("if there's a nonexistent pubkey", func() {
			It("should return a list containing 0 disallowed MixNodePresence objects", func() {
				topology := models.Topology{
					MixNodes: []models.MixNodePresence{mixpresence1, mixpresence2},
				}
				mockDb.On("ListDisallowed").Return([]string{"foomp"})
				mockDb.On("Topology").Return(topology)

				expectedDisallowed := []models.MixNodePresence{}

				response := serv.ListDisallowed()
				mockDb.AssertCalled(GinkgoT(), "ListDisallowed")
				mockDb.AssertCalled(GinkgoT(), "Topology")
				assert.Equal(GinkgoT(), expectedDisallowed, response)
			})
		})
	})
})
