package healthcheck

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

// controller is the presence controller
type controller struct{}

// Controller is the presence controller
type Controller interface {
	HealthCheck(c *gin.Context)
	RegisterRoutes(router *gin.Engine)
}

// New returns a new pki.Controller
func New() Controller {
	return &controller{}
}

func (controller *controller) RegisterRoutes(router *gin.Engine) {
	router.GET("/api/healthcheck", controller.HealthCheck)
}

// HealthCheck ...
// @Summary Lets the directory server tell the world it's alive.
// @Description Returns a 200 if the directory server is available. Good route to use for automated monitoring.
// @ID healthCheck
// @Accept  json
// @Produce  json
// @Tags healthcheck
// @Success 200
// @Router /api/healthcheck [get]
func (controller *controller) HealthCheck(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"ok": true})
}
