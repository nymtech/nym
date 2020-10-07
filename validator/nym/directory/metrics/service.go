package metrics

import (
	"encoding/json"
	"fmt"

	"github.com/BorisBorshevsky/timemock"
	"github.com/nymtech/nym-directory/models"
	"github.com/nymtech/nym-directory/server/websocket"
)

// Service struct
type Service struct {
	db  IDb
	hub websocket.IHub
}

// IService defines the REST service interface for metrics.
type IService interface {
	CreateMixMetric(metric models.MixMetric)
	List() []models.PersistedMixMetric
}

// NewService constructor
func NewService(db IDb, hub websocket.IHub) *Service {
	return &Service{
		db:  db,
		hub: hub,
	}
}

// CreateMixMetric adds a new PersistedMixMetric in the database.
func (service *Service) CreateMixMetric(metric models.MixMetric) {
	persist := models.PersistedMixMetric{
		MixMetric: metric,
		Timestamp: timemock.Now().UnixNano(),
	}
	service.db.Add(persist)

	b, err := json.Marshal(persist)
	if err != nil {
		fmt.Println(err)
		return
	}
	service.hub.Notify(b)

}

// List lists all mix metrics in the database
func (service *Service) List() []models.PersistedMixMetric {
	return service.db.List()
}
