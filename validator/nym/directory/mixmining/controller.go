package mixmining

import (
	"net/http"
	"strings"

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
	CreateMixStatus(c *gin.Context)
	RegisterRoutes(router *gin.Engine)
}

// New returns a new mixmining.Controller
func New(cfg Config) Controller {
	return &controller{cfg.Service, cfg.Sanitizer}
}

func (controller *controller) RegisterRoutes(router *gin.Engine) {
	router.POST("/api/mixmining", controller.CreateMixStatus)
	router.GET("/api/mixmining/:pubkey/history", controller.ListMeasurements)
	router.GET("/api/mixmining/:pubkey/report", controller.GetMixStatusReport)
}

// ListMeasurements lists mixnode statuses
// @Summary Lists mixnode activity
// @Description Lists all mixnode statuses for a given node pubkey
// @ID listMixStatuses
// @Accept  json
// @Produce  json
// @Tags mixmining
// @Param pubkey path string true "Mixnode Pubkey"
// @Success 200 {array} models.MixStatus
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/mixmining/{pubkey}/history [get]
func (controller *controller) ListMeasurements(c *gin.Context) {
	pubkey := c.Param("pubkey")
	measurements := controller.service.List(pubkey)
	c.JSON(http.StatusOK, measurements)
}

// CreateMixStatus ...
// @Summary Lets the network monitor create a new uptime status for a mix
// @Description Nym network monitor sends packets through the system and checks if they make it. The network monitor then hits this method to report whether the node was up at a given time.
// @ID addMixStatus
// @Accept  json
// @Produce  json
// @Tags mixmining
// @Param   object      body   models.MixStatus     true  "object"
// @Success 201
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/mixmining [post]
func (controller *controller) CreateMixStatus(c *gin.Context) {
	remoteIP := strings.Split((c.Request.RemoteAddr), ":")[0]
	if remoteIP != "127.0.0.1" {
		c.JSON(http.StatusForbidden, gin.H{"error": "forbidden"})
		return
	}
	var status models.MixStatus
	if err := c.ShouldBindJSON(&status); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	println("MADE IT HERE")
	sanitized := controller.sanitizer.Sanitize(status)
	persisted := controller.service.CreateMixStatus(sanitized)
	controller.service.SaveStatusReport(persisted)
	c.JSON(http.StatusCreated, gin.H{"ok": true})
}

// GetMixStatusReport ...
// @Summary Retrieves a summary report of historical mix status
// @Description Provides summary uptime statistics for last 5 minutes, day, week, and month
// @ID getMixStatusReport
// @Accept  json
// @Produce  json
// @Tags mixmining
// @Param pubkey path string true "Mixnode Pubkey"
// @Success 200
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/mixmining/{pubkey}/report [get]
func (controller *controller) GetMixStatusReport(c *gin.Context) {
	pubkey := c.Param("pubkey")
	report := controller.service.GetStatusReport(pubkey)
	if (report == models.MixStatusReport{}) {
		c.JSON(http.StatusNotFound, gin.H{"error": "not found"})
	}
	c.JSON(http.StatusOK, report)
}
