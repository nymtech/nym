package mixmining

import (
	"github.com/BorisBorshevsky/timemock"
	"github.com/nymtech/nym-directory/mixmining/mocks"
	"github.com/nymtech/nym-directory/models"
	. "github.com/onsi/ginkgo"
	"gotest.tools/assert"
)

// Some fixtures data to dry up tests a bit

// A slice of IPv4 mix statuses with 2 ups and 1 down during the past day
func twoUpOneDown() []models.PersistedMixStatus {
	db := []models.PersistedMixStatus{}
	var status = persistedStatus()

	booltrue := true
	status.PubKey = "key1"
	status.IPVersion = "4"
	status.Up = &booltrue

	status.Timestamp = minutesAgo(5)
	db = append(db, status)

	status.Timestamp = minutesAgo(10)
	db = append(db, status)

	boolfalse := false
	status.Timestamp = minutesAgo(15)
	status.Up = &boolfalse
	db = append(db, status)

	return db
}

func persistedStatus() models.PersistedMixStatus {
	mixStatus := status()
	persisted := models.PersistedMixStatus{
		MixStatus: mixStatus,
		Timestamp: Now(),
	}
	return persisted
}

func status() models.MixStatus {
	boolfalse := false
	return models.MixStatus{
		PubKey:    "key1",
		IPVersion: "4",
		Up:        &boolfalse,
	}
}

// A version of now with a frozen shared clock so we can have determinate time-based tests
func Now() int64 {
	now := timemock.Now()
	timemock.Freeze(now) //time is frozen
	nanos := now.UnixNano()
	return nanos
}

var _ = Describe("mixmining.Service", func() {
	var mockDb mocks.IDb
	var status1 models.MixStatus
	var status2 models.MixStatus
	var persisted1 models.PersistedMixStatus
	var persisted2 models.PersistedMixStatus

	var serv Service

	boolfalse := false
	booltrue := true

	status1 = models.MixStatus{
		PubKey:    "key1",
		IPVersion: "4",
		Up:        &boolfalse,
	}

	persisted1 = models.PersistedMixStatus{
		MixStatus: status1,
		Timestamp: Now(),
	}

	status2 = models.MixStatus{
		PubKey:    "key2",
		IPVersion: "6",
		Up:        &booltrue,
	}

	persisted2 = models.PersistedMixStatus{
		MixStatus: status2,
		Timestamp: Now(),
	}

	downer := persisted1
	downer.MixStatus.Up = &boolfalse

	upper := persisted1
	upper.MixStatus.Up = &booltrue

	persistedList := []models.PersistedMixStatus{persisted1, persisted2}
	emptyList := []models.PersistedMixStatus{}

	BeforeEach(func() {
		mockDb = *new(mocks.IDb)
		serv = *NewService(&mockDb)
	})

	Describe("Adding a mix status and creating a new summary report for a node", func() {
		Context("when no statuses have yet been saved", func() {
			It("should add a PersistedMixStatus to the db and save the new report", func() {

				mockDb.On("Add", persisted1)

				serv.CreateMixStatus(status1)
				mockDb.AssertCalled(GinkgoT(), "Add", persisted1)
			})
		})
	})
	Describe("Listing mix statuses", func() {
		Context("when receiving a list request", func() {
			It("should call to the Db", func() {
				mockDb.On("List", persisted1.PubKey, 1000).Return(persistedList)

				result := serv.List(persisted1.PubKey)

				mockDb.AssertCalled(GinkgoT(), "List", persisted1.PubKey, 1000)
				assert.Equal(GinkgoT(), persistedList[0].MixStatus.PubKey, result[0].MixStatus.PubKey)
				assert.Equal(GinkgoT(), persistedList[1].MixStatus.PubKey, result[1].MixStatus.PubKey)
			})
		})
	})

	Describe("Calculating uptime", func() {
		Context("when no statuses exist yet", func() {
			It("should return 0", func() {
				mockDb.On("ListDateRange", "key1", "4", daysAgo(30), now()).Return(emptyList)

				uptime := serv.CalculateUptime(persisted1.PubKey, persisted1.IPVersion, daysAgo(30))
				assert.Equal(GinkgoT(), 0, uptime)
			})

		})
		Context("when 2 ups and 1 down exist in the given time period", func() {
			It("should return 66", func() {
				mockDb.On("ListDateRange", "key1", "4", daysAgo(1), now()).Return(twoUpOneDown())

				uptime := serv.CalculateUptime("key1", "4", daysAgo(1))
				expected := 66 // percent
				assert.Equal(GinkgoT(), expected, uptime)
			})
		})
	})

	Describe("Saving a mix status report", func() {
		BeforeEach(func() {
			mockDb = *new(mocks.IDb)
			serv = *NewService(&mockDb)
		})
		Context("when 1 down status exists", func() {
			BeforeEach(func() {
				oneDown := []models.PersistedMixStatus{downer}
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, minutesAgo(5), now()).Return(oneDown)
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, minutesAgo(60), now()).Return(oneDown)
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(1), now()).Return(oneDown)
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(7), now()).Return(oneDown)
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(30), now()).Return(oneDown)
			})
			Context("this one *must be* a downer, so calculate using it", func() {
				BeforeEach(func() {
					mockDb.On("LoadReport", downer.PubKey).Return(models.MixStatusReport{}) // TODO: Mockery isn't happy returning an untyped nil, so I've had to sub in a blank `models.MixStatusReport{}`. It will actually return a nil.
					expectedSave := models.MixStatusReport{
						PubKey:           downer.PubKey,
						MostRecentIPV4:   false,
						Last5MinutesIPV4: 0,
						LastHourIPV4:     0,
						LastDayIPV4:      0,
						LastWeekIPV4:     0,
						LastMonthIPV4:    0,
						MostRecentIPV6:   false,
						Last5MinutesIPV6: 0,
						LastHourIPV6:     0,
						LastDayIPV6:      0,
						LastWeekIPV6:     0,
						LastMonthIPV6:    0,
					}
					mockDb.On("SaveMixStatusReport", expectedSave)
				})
				It("should save the initial report, all statuses will be set to down", func() {
					result := serv.SaveStatusReport(downer)
					assert.Equal(GinkgoT(), 0, result.Last5MinutesIPV4)
					assert.Equal(GinkgoT(), 0, result.LastHourIPV4)
					assert.Equal(GinkgoT(), 0, result.LastDayIPV4)
					assert.Equal(GinkgoT(), 0, result.LastWeekIPV4)
					assert.Equal(GinkgoT(), 0, result.LastMonthIPV4)
					mockDb.AssertExpectations(GinkgoT())
				})
			})

		})
		Context("when 1 up status exists", func() {
			BeforeEach(func() {
				oneUp := []models.PersistedMixStatus{upper}
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, minutesAgo(5), now()).Return(oneUp)
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, minutesAgo(60), now()).Return(oneUp)
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(1), now()).Return(oneUp)
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(7), now()).Return(oneUp)
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(30), now()).Return(oneUp)
			})
			Context("this one *must be* an upper, so calculate using it", func() {
				BeforeEach(func() {
					oneDown := []models.PersistedMixStatus{downer}
					mockDb.On("ListDateRange", upper.PubKey, upper.IPVersion, minutesAgo(5), now()).Return(oneDown)
					mockDb.On("ListDateRange", upper.PubKey, upper.IPVersion, minutesAgo(60), now()).Return(oneDown)
					mockDb.On("ListDateRange", upper.PubKey, upper.IPVersion, daysAgo(1), now()).Return(oneDown)
					mockDb.On("ListDateRange", upper.PubKey, upper.IPVersion, daysAgo(7), now()).Return(oneDown)
					mockDb.On("ListDateRange", upper.PubKey, upper.IPVersion, daysAgo(30), now()).Return(oneDown)
					mockDb.On("LoadReport", upper.PubKey).Return(models.MixStatusReport{}) // TODO: Mockery isn't happy returning an untyped nil, so I've had to sub in a blank `models.MixStatusReport{}`. It will actually return a nil.
					expectedSave := models.MixStatusReport{
						PubKey:           upper.PubKey,
						MostRecentIPV4:   true,
						Last5MinutesIPV4: 100,
						LastHourIPV4:     100,
						LastDayIPV4:      100,
						LastWeekIPV4:     100,
						LastMonthIPV4:    100,
						MostRecentIPV6:   false,
						Last5MinutesIPV6: 0,
						LastHourIPV6:     0,
						LastDayIPV6:      0,
						LastWeekIPV6:     0,
						LastMonthIPV6:    0,
					}
					mockDb.On("SaveMixStatusReport", expectedSave)
				})
				It("should save the initial report, all statuses will be set to up", func() {
					result := serv.SaveStatusReport(upper)
					assert.Equal(GinkgoT(), true, result.MostRecentIPV4)
					assert.Equal(GinkgoT(), 100, result.Last5MinutesIPV4)
					assert.Equal(GinkgoT(), 100, result.LastHourIPV4)
					assert.Equal(GinkgoT(), 100, result.LastDayIPV4)
					assert.Equal(GinkgoT(), 100, result.LastWeekIPV4)
					assert.Equal(GinkgoT(), 100, result.LastMonthIPV4)
					mockDb.AssertExpectations(GinkgoT())
				})
			})
		})

		Context("when 2 up statuses exist for the last 5 minutes already and we just added a down", func() {
			BeforeEach(func() {
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, minutesAgo(5), now()).Return(twoUpOneDown())
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, minutesAgo(60), now()).Return(twoUpOneDown())
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(1), now()).Return(twoUpOneDown())
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(7), now()).Return(twoUpOneDown())
				mockDb.On("ListDateRange", downer.PubKey, downer.IPVersion, daysAgo(30), now()).Return(twoUpOneDown())
			})
			It("should save the report", func() {
				initialState := models.MixStatusReport{
					PubKey:           downer.PubKey,
					MostRecentIPV4:   true,
					Last5MinutesIPV4: 100,
					LastHourIPV4:     100,
					LastDayIPV4:      100,
					LastWeekIPV4:     100,
					LastMonthIPV4:    100,
					MostRecentIPV6:   false,
					Last5MinutesIPV6: 0,
					LastHourIPV6:     0,
					LastDayIPV6:      0,
					LastWeekIPV6:     0,
					LastMonthIPV6:    0,
				}

				expectedAfterUpdate := models.MixStatusReport{
					PubKey:           downer.PubKey,
					MostRecentIPV4:   false,
					Last5MinutesIPV4: 66,
					LastHourIPV4:     66,
					LastDayIPV4:      66,
					LastWeekIPV4:     66,
					LastMonthIPV4:    66,
					MostRecentIPV6:   false,
					Last5MinutesIPV6: 0,
					LastHourIPV6:     0,
					LastDayIPV6:      0,
					LastWeekIPV6:     0,
					LastMonthIPV6:    0,
				}
				mockDb.On("LoadReport", downer.PubKey).Return(initialState)
				mockDb.On("SaveMixStatusReport", expectedAfterUpdate)
				updatedStatus := serv.SaveStatusReport(downer)
				assert.Equal(GinkgoT(), expectedAfterUpdate, updatedStatus)
				mockDb.AssertExpectations(GinkgoT())
			})
		})
	})

	Describe("Getting a mix status report", func() {
		Context("When no saved report exists for a pubkey", func() {
			It("should return an empty report", func() {
				mockDb = *new(mocks.IDb)
				serv = *NewService(&mockDb)

				blank := models.MixStatusReport{}
				mockDb.On("LoadReport", "superkey").Return(blank)

				report := serv.GetStatusReport("superkey")
				assert.Equal(GinkgoT(), blank, report)
			})
		})
		Context("When a saved report exists for a pubkey", func() {
			It("should return the report", func() {
				mockDb = *new(mocks.IDb)
				serv = *NewService(&mockDb)

				perfect := models.MixStatusReport{
					PubKey:           "superkey",
					MostRecentIPV4:   true,
					Last5MinutesIPV4: 100,
					LastHourIPV4:     100,
					LastDayIPV4:      100,
					LastWeekIPV4:     100,
					LastMonthIPV4:    100,
					MostRecentIPV6:   true,
					Last5MinutesIPV6: 100,
					LastHourIPV6:     100,
					LastDayIPV6:      100,
					LastWeekIPV6:     100,
					LastMonthIPV6:    100,
				}
				mockDb.On("LoadReport", "superkey").Return(perfect)

				report := serv.GetStatusReport("superkey")
				assert.Equal(GinkgoT(), perfect, report)
			})
		})
	})
})
