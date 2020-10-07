package metrics

import (
	"sync"
	"time"

	"github.com/nymtech/nym-directory/models"
)

// IDb holds metrics information
type IDb interface {
	Add(models.PersistedMixMetric)
	List() []models.PersistedMixMetric
}

// Db holds data for metrics
type Db struct {
	sync.Mutex
	incomingMetrics []models.PersistedMixMetric
	mixMetrics      []models.PersistedMixMetric
	ticker          *time.Ticker
}

// NewDb constructor
func NewDb() *Db {
	ticker := time.NewTicker(3 * time.Second)

	d := Db{
		incomingMetrics: []models.PersistedMixMetric{},
		mixMetrics:      []models.PersistedMixMetric{},
	}
	d.ticker = ticker
	go dbCleaner(ticker, &d)

	return &d
}

// Add adds a models.PersistedMixMetric to the database
func (db *Db) Add(metric models.PersistedMixMetric) {
	db.Lock()
	defer db.Unlock()
	db.incomingMetrics = append(db.incomingMetrics, metric)
}

// List returns all models.PersistedMixMetric in the database
func (db *Db) List() []models.PersistedMixMetric {
	db.Lock()
	defer db.Unlock()
	return db.mixMetrics
}

// dbCleaner periodically clears the database
func dbCleaner(ticker *time.Ticker, database *Db) {
	for {
		select {
		case <-ticker.C:
			database.clear()
		}
	}
}

// clear kills any stale metrics info
//
// This may look a little weird, but there's a logic to it.
//
// If we have only one array holding metrics, incoming metrics get stacked up
// for a while, and then all destroyed at once, so  the list we can provider
// starts empty, swells, then becomes empty again. This doesn't offer clients
// a consistent view of what happened.
//
// Instead we Add() to an `incomingMixMetrics` slice, and read from the
// `mixMetrics` slice. When we clear the db, we can transfer everything from
// `incoming` to `mixMetrics` and have a full list, clearing incoming.
// This way we can offer a consistent view of what happened
// over any individual bit of time.
func (db *Db) clear() {
	db.Lock()
	defer db.Unlock()
	db.mixMetrics = db.incomingMetrics
	db.incomingMetrics = db.incomingMetrics[:0]
}
