package metrics

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/nymtech/nym-directory/models"
)

// Config for this controller
type Config struct {
	Sanitizer Sanitizer
	Service   IService
}

// controller is the metrics controller
type controller struct {
	service   IService
	sanitizer Sanitizer
}

// Controller ...
type Controller interface {
	CreateMixMetric(c *gin.Context)
	RegisterRoutes(router *gin.Engine)
}

// New returns a new metrics.Controller...
func New(cfg Config) Controller {
	return &controller{cfg.Service, cfg.Sanitizer}
}

func (controller *controller) RegisterRoutes(router *gin.Engine) {
	router.POST("/api/metrics/mixes", controller.CreateMixMetric)
	router.GET("/api/metrics/mixes", controller.ListMixMetrics)
}

// CreateMixMetric ...
// @Summary Create a metric detailing how many messages a given mixnode sent and received
// @Description For demo and debug purposes it gives us the ability to generate useful visualisations of network traffic.
// @ID createMixMetric
// @Accept  json
// @Produce  json
// @Tags metrics
// @Param   object      body   models.MixMetric     true  "object"
// @Success 201
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/metrics/mixes [post]
func (controller *controller) CreateMixMetric(c *gin.Context) {
	var metric models.MixMetric
	if err := c.ShouldBindJSON(&metric); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	sanitized := controller.sanitizer.Sanitize(metric)
	controller.service.CreateMixMetric(sanitized)
	c.JSON(http.StatusCreated, gin.H{"ok": true})
}

// ListMixMetrics lists mixnode activity
// @Summary Lists mixnode activity in the past 3 seconds
// @Description For demo and debug purposes it gives us the ability to generate useful visualisations of network traffic.
// @ID listMixMetrics
// @Accept  json
// @Produce  json
// @Tags metrics
// Param   object      body   models.ObjectRequest     true  "object"
// @Success 200 {array} models.MixMetric
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/metrics/mixes [get]
func (controller *controller) ListMixMetrics(c *gin.Context) {
	metrics := controller.service.List()
	c.JSON(http.StatusOK, metrics)
}
