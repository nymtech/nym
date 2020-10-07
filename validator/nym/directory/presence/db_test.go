package presence

import (
	"time"

	"github.com/BorisBorshevsky/timemock"
	"github.com/nymtech/nym-directory/models"
	. "github.com/onsi/ginkgo"
	"github.com/stretchr/testify/assert"
)

var _ = Describe("Presence Db", func() {
	Describe("listing network topology", func() {
		Context("when no presence has been registered by any node", func() {
			It("should return an empty topology object", func() {
				db := NewDb()
				assert.Len(GinkgoT(), db.Topology().CocoNodes, 0)
				assert.Len(GinkgoT(), db.Topology().MixNodes, 0)
				assert.Len(GinkgoT(), db.Topology().MixProviderNodes, 0)
				assert.NotNil(GinkgoT(), db.Topology().CocoNodes)
				assert.NotNil(GinkgoT(), db.Topology().MixNodes)
				assert.NotNil(GinkgoT(), db.Topology().MixProviderNodes)
			})
		})
	})
	Describe("for coconodes", func() {
		var (
			presence1 models.CocoPresence
			presence2 models.CocoPresence
		)
		var db *Db
		BeforeEach(func() {
			db = NewDb()

			// Set up fixtures
			var coco1 = models.CocoHostInfo{
				HostInfo: models.HostInfo{
					Host:   "foo.com:8000",
					PubKey: "pubkey1",
				},
				Type: "foo",
			}
			presence1 = models.CocoPresence{
				CocoHostInfo: coco1,
				LastSeen:     timemock.Now().UnixNano(),
			}

			var coco2 = models.CocoHostInfo{
				HostInfo: models.HostInfo{
					Host:   "foo.com:8000",
					PubKey: "pubkey2",
				},
				Type: "foo",
			}
			presence2 = models.CocoPresence{
				CocoHostInfo: coco2,
				LastSeen:     timemock.Now().UnixNano(),
			}
		})
		Describe("adding presence", func() {
			Context("1st presence", func() {
				It("adds properly", func() {
					db.AddCoco(presence1)
				})
			})
		})
		Context("adding two coconode presences", func() {
			It("returns the map correctly", func() {
				db.AddCoco(presence1)
				db.AddCoco(presence2)
				assert.Len(GinkgoT(), db.Topology().CocoNodes, 2)
			})
			It("contains the correct presences", func() {
				db.AddCoco(presence1)
				db.AddCoco(presence2)
				assert.Contains(GinkgoT(), db.Topology().CocoNodes, presence1)
				assert.Contains(GinkgoT(), db.Topology().CocoNodes, presence2)
			})
		})
		Describe("Presences", func() {
			Context("more than 20 seconds old", func() {
				It("are not returned in the topology", func() {
					oldtime := timemock.Now().Add(time.Duration(-20 * time.Second)).UnixNano()
					presence1.LastSeen = oldtime
					db.AddCoco(presence1)
					db.AddCoco(presence2)
					assert.Len(GinkgoT(), db.Topology().CocoNodes, 1)
					assert.Equal(GinkgoT(), presence2, db.Topology().CocoNodes[0])
				})
			})
		})

	})
	Describe("for mixnodes", func() {
		var (
			presence1 models.MixNodePresence
			presence2 models.MixNodePresence
		)
		var db *Db
		BeforeEach(func() {
			db = NewDb()

			// Set up fixtures
			var mix1 = models.MixHostInfo{
				HostInfo: models.HostInfo{
					Host:   "foo.com:8000",
					PubKey: "pubkey1",
				},
				Layer: 1,
			}
			presence1 = models.MixNodePresence{
				MixHostInfo: mix1,
				LastSeen:    timemock.Now().UnixNano(),
			}

			var mix2 = models.MixHostInfo{
				HostInfo: models.HostInfo{
					Host:   "bar.com:8000",
					PubKey: "pubkey2",
				},
				Layer: 2,
			}
			presence2 = models.MixNodePresence{
				MixHostInfo: mix2,
				LastSeen:    timemock.Now().UnixNano(),
			}
		})
		Describe("adding mixnode presence", func() {
			Context("1st presence", func() {
				It("returns the map correctly", func() {
					db.AddMix(presence1)
					assert.Len(GinkgoT(), db.Topology().MixNodes, 1)
				})
				It("gets the presence by its public key", func() {
					db.AddMix(presence1)
					assert.Equal(GinkgoT(), presence1, db.Topology().MixNodes[0])
				})
			})
			Context("adding two mixnode presences", func() {
				It("returns the map correctly", func() {
					db.AddMix(presence1)
					db.AddMix(presence2)
					assert.Len(GinkgoT(), db.Topology().MixNodes, 2)
				})
				It("contains the correct presences", func() {
					db.AddMix(presence1)
					db.AddMix(presence2)
					assert.Contains(GinkgoT(), db.Topology().MixNodes, presence1)
					assert.Contains(GinkgoT(), db.Topology().MixNodes, presence2)
				})
			})
			Describe("Presences", func() {
				Context("more than 20 seconds old", func() {
					It("are not returned in the topology", func() {
						oldtime := timemock.Now().Add(time.Duration(-20 * time.Second)).UnixNano()
						presence1.LastSeen = oldtime
						db.AddMix(presence1)
						db.AddMix(presence2)
						assert.Len(GinkgoT(), db.Topology().MixNodes, 1)
						assert.Equal(GinkgoT(), presence2, db.Topology().MixNodes[0])
					})
				})
			})
		})
	})

	Describe("for mixnode providers", func() {
		var (
			presence1 models.MixProviderPresence
			presence2 models.MixProviderPresence
		)
		var db *Db
		BeforeEach(func() {
			db = NewDb()

			// Set up fixtures
			var mix1 = models.MixProviderHostInfo{
				MixnetListener: "foo.com:8000",
				ClientListener: "foo.com:8001",
				PubKey:         "pubkey1",
			}
			presence1 = models.MixProviderPresence{
				MixProviderHostInfo: mix1,
				LastSeen:            timemock.Now().UnixNano(),
			}

			var mix2 = models.MixProviderHostInfo{
				MixnetListener: "foo.com:8000",
				ClientListener: "foo.com:8001",
				PubKey:         "pubkey2",
			}
			presence2 = models.MixProviderPresence{
				MixProviderHostInfo: mix2,
				LastSeen:            timemock.Now().UnixNano(),
			}
		})
		Describe("adding mixnode presence", func() {
			Context("1st presence", func() {
				It("returns the map correctly", func() {
					db.AddMixProvider(presence1)
					assert.Len(GinkgoT(), db.Topology().MixProviderNodes, 1)
				})
				It("gets the presence by its public key", func() {
					db.AddMixProvider(presence1)
					assert.Equal(GinkgoT(), presence1, db.Topology().MixProviderNodes[0])
				})
			})
			Context("adding two mixnode presences", func() {
				It("returns the map correctly", func() {
					db.AddMixProvider(presence1)
					db.AddMixProvider(presence2)
					assert.Len(GinkgoT(), db.Topology().MixProviderNodes, 2)
				})
				It("contains the correct presences", func() {
					db.AddMixProvider(presence1)
					db.AddMixProvider(presence2)
					assert.Contains(GinkgoT(), db.Topology().MixProviderNodes, presence1)
					assert.Contains(GinkgoT(), db.Topology().MixProviderNodes, presence2)
				})
			})
			Describe("Presences", func() {
				Context("more than 20 seconds old", func() {
					It("are not returned in the topology", func() {
						oldtime := timemock.Now().Add(time.Duration(-20 * time.Second)).UnixNano()
						presence1.LastSeen = oldtime
						db.AddMixProvider(presence1)
						db.AddMixProvider(presence2)
						assert.Len(GinkgoT(), db.Topology().MixProviderNodes, 1)
						assert.Equal(GinkgoT(), presence2, db.Topology().MixProviderNodes[0])
					})
				})
			})
		})
	})

	Describe("allowing and disallowing mixnodes", func() {
		Context("adding a disallowed pubkey", func() {
			It("should add the pubkey and return it in the disallowed list", func() {
				// initial state
				db := NewDb()
				assert.Len(GinkgoT(), db.ListDisallowed(), 0)

				pubkey := "abc123"

				// disallowing
				db.Disallow(pubkey)
				assert.Len(GinkgoT(), db.ListDisallowed(), 1)
				assert.Contains(GinkgoT(), db.ListDisallowed(), pubkey)

				// allowing
				db.Allow(pubkey)
				assert.Len(GinkgoT(), db.ListDisallowed(), 0)
				assert.NotContains(GinkgoT(), db.ListDisallowed(), pubkey)

			})
		})

		Context("adding a disallowed base64 pubkey", func() {
			It("should add the pubkey and return it in the disallowed list", func() {
				// initial state
				db := NewDb()
				assert.Len(GinkgoT(), db.ListDisallowed(), 0)

				pubkey := "bzWdTz9E-VD9UWnvDSz5-qEs_lOQ_7PA7cOp9wIwzxI="

				// disallowing
				db.Disallow(pubkey)
				assert.Len(GinkgoT(), db.ListDisallowed(), 1)
				assert.Contains(GinkgoT(), db.ListDisallowed(), pubkey)

				// allowing
				db.Allow(pubkey)
				assert.Len(GinkgoT(), db.ListDisallowed(), 0)
				assert.NotContains(GinkgoT(), db.ListDisallowed(), pubkey)
			})
		})
	})

	Describe("disallowing mixnodes", func() {
		Context("twice", func() {
			It("should not add a second instance of the already disallowed pubkey", func() {
				pubkey := "abc123"
				db := NewDb()
				assert.Len(GinkgoT(), db.ListDisallowed(), 0)
				db.Disallow(pubkey)
				assert.Len(GinkgoT(), db.ListDisallowed(), 1)
				db.Disallow(pubkey)
				assert.Len(GinkgoT(), db.ListDisallowed(), 1)
				assert.Contains(GinkgoT(), db.ListDisallowed(), pubkey)
			})
		})
	})
})
