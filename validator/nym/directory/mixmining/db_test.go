package mixmining

import (
	"github.com/nymtech/nym-directory/mixmining/fixtures"
	"github.com/nymtech/nym-directory/models"
	. "github.com/onsi/ginkgo"
	"github.com/stretchr/testify/assert"
)

var _ = Describe("The mixmining db", func() {
	Describe("Constructing a NewDb", func() {
		Context("a new db", func() {
			It("should have no mixmining statuses", func() {
				db := NewDb()
				db.orm.Exec("DELETE FROM persisted_mix_statuses")
				assert.Len(GinkgoT(), db.List("foo", 5), 0)
			})
		})
	})

	Describe("adding and retrieving measurements", func() {
		Context("a new db", func() {
			It("should add measurements to the db, with a timestamp, and be able to retrieve them afterwards", func() {
				db := NewDb()
				db.orm.Exec("DELETE FROM persisted_mix_statuses")
				status := fixtures.GoodPersistedMixStatus()

				// add one
				db.Add(status)
				measurements := db.List(status.PubKey, 5)
				assert.Len(GinkgoT(), measurements, 1)
				assert.Equal(GinkgoT(), status, measurements[0])

				// add another
				db.Add(status)
				measurements = db.List(status.PubKey, 5)
				assert.Len(GinkgoT(), measurements, 2)
				assert.Equal(GinkgoT(), status, measurements[0])
				assert.Equal(GinkgoT(), status, measurements[1])
			})
		})
	})

	Describe("listing mix statuses within a date range", func() {
		Context("for an empty db", func() {
			It("should return an empty slice", func() {
				db := NewDb()
				db.orm.Exec("DELETE FROM persisted_mix_statuses")
				assert.Len(GinkgoT(), db.ListDateRange("foo", "6", 1, 1), 0)
			})
		})
		Context("when one status exists in the range and one outside", func() {
			It("should return only the status within the range", func() {
				db := NewDb()
				db.orm.Exec("DELETE FROM persisted_mix_statuses")
				data := fixtures.GoodMixStatus()
				statusInRange := models.PersistedMixStatus{
					MixStatus: data,
					Timestamp: 500,
				}
				statusOutOfRange := models.PersistedMixStatus{
					MixStatus: data,
					Timestamp: 1000,
				}
				db.Add(statusInRange)
				db.Add(statusOutOfRange)

				result := db.ListDateRange(data.PubKey, "6", 0, 500)
				assert.Len(GinkgoT(), result, 1)
				assert.Equal(GinkgoT(), statusInRange, result[0])
			})
		})
		Context("when one Ipv4 status exists in the range and one outside, with an IPv6 status also in range, when searching for IPv4", func() {
			It("should return only the status within the range", func() {
				db := NewDb()
				db.orm.Exec("DELETE FROM persisted_mix_statuses")
				ip4data := fixtures.GoodMixStatus()
				ip4data.IPVersion = "4"

				ip6data := fixtures.GoodMixStatus()
				ip6data.IPVersion = "6"
				ip4statusInRange := models.PersistedMixStatus{
					MixStatus: ip4data,
					Timestamp: 500,
				}
				ip6statusInRange := models.PersistedMixStatus{
					MixStatus: ip6data,
					Timestamp: 500,
				}
				ip4statusOutOfRange := models.PersistedMixStatus{
					MixStatus: ip4data,
					Timestamp: 1000,
				}
				db.Add(ip4statusInRange)
				db.Add(ip6statusInRange)
				db.Add(ip4statusOutOfRange)

				result := db.ListDateRange(ip4statusInRange.PubKey, "4", 0, 500)
				assert.Len(GinkgoT(), result, 1)
				assert.Equal(GinkgoT(), ip4statusInRange, result[0])
			})
		})
	})

	Describe("listing mix statuses with a limit", func() {
		Context("for an empty db", func() {
			It("should return an empty slice", func() {
				db := NewDb()
				defer db.orm.Exec("DELETE FROM persisted_mix_statuses")
				assert.Len(GinkgoT(), db.List("foo", 5), 0)
			})
		})
	})

	Describe("saving a mix status report", func() {
		Context("for an empty db", func() {
			It("should save and reload the report", func() {
				db := NewDb()
				db.orm.Exec("DELETE FROM mix_status_reports")
				newReport := models.MixStatusReport{
					PubKey:           "key",
					MostRecentIPV4:   true,
					Last5MinutesIPV4: 5,
					LastHourIPV4:     10,
					LastDayIPV4:      15,
					LastWeekIPV4:     20,
					LastMonthIPV4:    25,
					MostRecentIPV6:   false,
					Last5MinutesIPV6: 30,
					LastHourIPV6:     40,
					LastDayIPV6:      50,
					LastWeekIPV6:     60,
					LastMonthIPV6:    70,
				}
				db.SaveMixStatusReport(newReport)
				saved := db.LoadReport(newReport.PubKey)
				assert.Equal(GinkgoT(), newReport, saved)
			})
		})
		Context("when saving a second time", func() {
			It("should re-save the original report, and not make a second copy", func() {
				db := NewDb()
				db.orm.Exec("DELETE FROM mix_status_reports")

				newReport := models.MixStatusReport{
					PubKey:           "key",
					MostRecentIPV4:   true,
					Last5MinutesIPV4: 5,
					LastHourIPV4:     10,
					LastDayIPV4:      15,
					LastWeekIPV4:     20,
					LastMonthIPV4:    25,
					MostRecentIPV6:   false,
					Last5MinutesIPV6: 30,
					LastHourIPV6:     40,
					LastDayIPV6:      50,
					LastWeekIPV6:     60,
					LastMonthIPV6:    70,
				}
				db.SaveMixStatusReport(newReport)

				var firstCount int64
				db.orm.Model(&models.MixStatusReport{}).Where("pub_key = ?", "key").Count(&firstCount)
				assert.Equal(GinkgoT(), int64(1), firstCount)

				report := db.LoadReport("key")
				report.Last5MinutesIPV4 = 666

				db.SaveMixStatusReport(report)

				var secondCount int64
				db.orm.Model(&models.MixStatusReport{}).Where("pub_key = ?", "key").Count(&secondCount)
				assert.Equal(GinkgoT(), int64(1), secondCount)

				reloadedReport := db.LoadReport("key")
				assert.Equal(GinkgoT(), 666, reloadedReport.Last5MinutesIPV4)
			})
		})
	})
})
