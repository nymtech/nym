package mixmining

import (
	"time"

	"github.com/BorisBorshevsky/timemock"
	"github.com/nymtech/nym-directory/models"
)

// Service struct
type Service struct {
	db IDb
}

// IService defines the REST service interface for metrics.
type IService interface {
	CreateMixStatus(metric models.MixStatus) models.PersistedMixStatus
	List(pubkey string) []models.PersistedMixStatus
	SaveStatusReport(status models.PersistedMixStatus) models.MixStatusReport
	GetStatusReport(pubkey string) models.MixStatusReport
}

// NewService constructor
func NewService(db IDb) *Service {
	return &Service{
		db: db,
	}
}

// CreateMixStatus adds a new PersistedMixStatus in the orm.
func (service *Service) CreateMixStatus(mixStatus models.MixStatus) models.PersistedMixStatus {
	persistedMixStatus := models.PersistedMixStatus{
		MixStatus: mixStatus,
		Timestamp: timemock.Now().UnixNano(),
	}
	service.db.Add(persistedMixStatus)
	return persistedMixStatus
}

// List lists the given number mix metrics
func (service *Service) List(pubkey string) []models.PersistedMixStatus {
	return service.db.List(pubkey, 1000)
}

// GetStatusReport gets a single MixStatusReport by node public key
func (service *Service) GetStatusReport(pubkey string) models.MixStatusReport {
	return service.db.LoadReport(pubkey)
}

// SaveStatusReport builds and saves a status report for a mixnode. The report can be updated once
// whenever we receive a new status, and the saved result can then be queried. This keeps us from
// having to build the report dynamically on every request at runtime.
func (service *Service) SaveStatusReport(status models.PersistedMixStatus) models.MixStatusReport {
	report := service.db.LoadReport(status.PubKey)
	report.PubKey = status.PubKey // crude, we do this in case it's a fresh struct returned from the db

	if status.IPVersion == "4" {
		report.MostRecentIPV4 = *status.Up
		report.Last5MinutesIPV4 = service.CalculateUptime(status.PubKey, "4", minutesAgo(5))
		report.LastHourIPV4 = service.CalculateUptime(status.PubKey, "4", minutesAgo(60))
		report.LastDayIPV4 = service.CalculateUptime(status.PubKey, "4", daysAgo(1))
		report.LastWeekIPV4 = service.CalculateUptime(status.PubKey, "4", daysAgo(7))
		report.LastMonthIPV4 = service.CalculateUptime(status.PubKey, "4", daysAgo(30))
	} else if status.IPVersion == "6" {
		report.MostRecentIPV6 = *status.Up
		report.Last5MinutesIPV6 = service.CalculateUptime(status.PubKey, "6", minutesAgo(5))
		report.LastHourIPV6 = service.CalculateUptime(status.PubKey, "6", minutesAgo(60))
		report.LastDayIPV6 = service.CalculateUptime(status.PubKey, "6", daysAgo(1))
		report.LastWeekIPV6 = service.CalculateUptime(status.PubKey, "6", daysAgo(7))
		report.LastMonthIPV6 = service.CalculateUptime(status.PubKey, "6", daysAgo(30))
	}
	service.db.SaveMixStatusReport(report)
	return report
}

// CalculateUptime calculates percentage uptime for a given node, protocol since a specific time
func (service *Service) CalculateUptime(pubkey string, ipVersion string, since int64) int {
	statuses := service.db.ListDateRange(pubkey, ipVersion, since, now())
	numStatuses := len(statuses)
	if numStatuses == 0 {
		return 0
	}
	up := 0
	for _, status := range statuses {
		if *status.Up {
			up = up + 1
		}
	}
	return service.calculatePercent(up, numStatuses)
}

func (service *Service) calculatePercent(num int, outOf int) int {
	return int(float32(num) / float32(outOf) * 100)
}

func now() int64 {
	return timemock.Now().UnixNano()
}

func daysAgo(days int) int64 {
	now := timemock.Now()
	return now.Add(time.Duration(-days) * time.Hour * 24).UnixNano()
}

func minutesAgo(minutes int) int64 {
	now := timemock.Now()
	return now.Add(time.Duration(-minutes) * time.Minute).UnixNano()
}

func secondsAgo(seconds int) int64 {
	now := timemock.Now()
	return now.Add(time.Duration(-seconds) * time.Second).UnixNano()
}
