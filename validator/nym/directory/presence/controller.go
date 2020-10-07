package presence

import (
	"net/http"

	"github.com/blang/semver/v4"
	"github.com/gin-gonic/gin"
	"github.com/nymtech/nym-directory/models"
)

// Config for this controller
type Config struct {
	CocoHostSanitizer        CocoHostSanitizer
	MixHostSanitizer         MixHostSanitizer
	MixNodeIDSanitizer       IMixNodeIDSanitizer
	MixProviderHostSanitizer MixProviderHostSanitizer
	GatewayHostSanitizer     GatewayHostSanitizer
	Service                  IService
}

// controller is the presence controller
type controller struct {
	service                  IService
	cocoHostSanitizer        CocoHostSanitizer
	mixHostSanitizer         MixHostSanitizer
	mixProviderHostSanitizer MixProviderHostSanitizer
	gatewayHostSanitizer     GatewayHostSanitizer
}

// Controller is the presence controller interface
type Controller interface {
	AddCocoNodePresence(c *gin.Context)
	AddMixNodePresence(c *gin.Context)
	Topology(c *gin.Context)
	RegisterRoutes(router *gin.Engine)
}

// New constructor
func New(cfg Config) Controller {
	return &controller{
		cfg.Service,
		cfg.CocoHostSanitizer,
		cfg.MixHostSanitizer,
		cfg.MixProviderHostSanitizer,
		cfg.GatewayHostSanitizer,
	}
}

// RegisterRoutes registers controller routes in Gin.
func (controller *controller) RegisterRoutes(router *gin.Engine) {
	router.POST("/api/presence/allow", controller.Allow)
	router.POST("/api/presence/coconodes", controller.AddCocoNodePresence)
	router.POST("/api/presence/disallow", controller.Disallow)
	router.GET("/api/presence/disallowed", controller.Disallowed)
	router.POST("/api/presence/mixnodes", controller.AddMixNodePresence)
	router.POST("/api/presence/gateways", controller.AddGatewayPresence)
	router.GET("/api/presence/topology", controller.Topology)
}

// AddMixNodePresence ...
// @Summary Lets mixnode a node tell the directory server it's alive
// @Description Nym mixnodes can ping this method to let the directory server know they're up. We can then use this info to create topologies of the overall Nym network.
// @ID addMixNode
// @Accept  json
// @Produce  json
// @Tags presence
// @Param   object      body   models.MixHostInfo     true  "object"
// @Success 201
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/presence/mixnodes [post]
func (controller *controller) AddMixNodePresence(c *gin.Context) {
	var mixHost models.MixHostInfo
	if err := c.ShouldBindJSON(&mixHost); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	sanitized := controller.mixHostSanitizer.Sanitize(mixHost)
	version, err := semver.Make(mixHost.Version)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	minVersion, _ := semver.Make("0.6.0")
	if version.LT(minVersion) {
		c.JSON(http.StatusUnprocessableEntity, gin.H{"error": "Minimum support mixnode version is 0.6.0"})
		return
	}

	controller.service.AddMixNodePresence(sanitized)
	c.JSON(http.StatusCreated, gin.H{"ok": true})
}

// AddCocoNodePresence ...
// @Summary Lets a coconut node tell the directory server it's alive
// @Description Nym Coconut nodes can ping this method to let the directory server know they're up. We can then use this info to create topologies of the overall Nym network.
// @ID addCocoNode
// @Accept  json
// @Produce  json
// @Tags presence
// @Param   object      body   models.CocoHostInfo     true  "object"
// @Success 201
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/presence/coconodes [post]
func (controller *controller) AddCocoNodePresence(c *gin.Context) {
	var cocoHost models.CocoHostInfo
	if err := c.ShouldBindJSON(&cocoHost); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	sanitized := controller.cocoHostSanitizer.Sanitize(cocoHost)
	controller.service.AddCocoNodePresence(sanitized, c.ClientIP())
	c.JSON(http.StatusCreated, gin.H{"ok": true})
}

// AddGatewayPresence ...
// @Summary Lets a gateway tell the directory server it's alive
// @Description Nym mix gateways can ping this method to let the directory server know they're up. We can then use this info to create topologies of the overall Nym network.
// @ID addGateway
// @Accept  json
// @Produce  json
// @Tags presence
// @Param   object      body   models.GatewayHostInfo     true  "object"
// @Success 201
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/presence/gateways [post]
func (controller *controller) AddGatewayPresence(c *gin.Context) {
	var gateway models.GatewayHostInfo
	if err := c.ShouldBindJSON(&gateway); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	sanitized := controller.gatewayHostSanitizer.Sanitize(gateway)
	controller.service.AddGatewayPresence(sanitized)
	c.JSON(http.StatusCreated, gin.H{"ok": true})
}

// Allow ...
// @Summary Removes a disallowed node from the disallowed nodes list
// @Description Sometimes when a node isn't working we need to temporarily remove it. This allows us to re-enable it once it's working again.
// @ID allow
// @Accept  json
// @Produce  json
// @Tags presence
// @Param   object      body   models.MixNodeID     true  "object"
// @Success 200
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/presence/allow [post]
func (controller *controller) Allow(c *gin.Context) {
	var node models.MixNodeID
	if err := c.ShouldBindJSON(&node); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	controller.service.Allow(node)
	c.JSON(http.StatusOK, gin.H{"ok": true})
}

// Disallow ...
// @Summary Takes a node out of the regular topology and puts it in the disallowed nodes list
// @Description Sometimes when a node isn't working we need to temporarily remove it from use so that it doesn't mess up QoS for the whole network.
// @ID disallow
// @Accept  json
// @Produce  json
// @Tags presence
// @Param   object      body   models.MixNodeID     true  "object"
// @Success 201
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/presence/disallow [post]
func (controller *controller) Disallow(c *gin.Context) {
	var node models.MixNodeID
	if err := c.ShouldBindJSON(&node); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	controller.service.Disallow(node)
	c.JSON(http.StatusCreated, gin.H{"ok": true})
}

// Disallowed ...
// @Summary Lists Nym mixnodes that are currently disallowed
// @Description Sometimes we need to take mixnodes out of the network for repair. This shows which ones are currently disallowed.
// @ID disallowed
// @Accept  json
// @Produce  json
// @Tags presence
// @Success 200 {array} models.MixNodePresence
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/presence/disallowed [get]
func (controller *controller) Disallowed(c *gin.Context) {
	disallowed := controller.service.ListDisallowed()
	c.JSON(http.StatusOK, disallowed)
}

// Topology ...
// @Summary Lists which Nym mixnodes, providers, gateways, and coconodes are alive
// @Description Nym nodes periodically ping the directory server to register that they're alive. This method provides a list of nodes which have been most recently seen.
// @ID topology
// @Accept  json
// @Produce  json
// @Tags presence
// @Success 200 {object} models.Topology
// @Failure 400 {object} models.Error
// @Failure 404 {object} models.Error
// @Failure 500 {object} models.Error
// @Router /api/presence/topology [get]
func (controller *controller) Topology(c *gin.Context) {
	topology := controller.service.Topology()
	c.JSON(http.StatusOK, topology)
}
