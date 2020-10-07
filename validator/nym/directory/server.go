package server

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/microcosm-cc/bluemonday"
	"github.com/nymtech/nym/validator/nym/directory/healthcheck"
	"github.com/nymtech/nym/validator/nym/directory/mixmining"
	"github.com/nymtech/nym/validator/nym/directory/presence"
	"github.com/nymtech/nym/validator/nym/directory/server/html"
	"github.com/nymtech/nym/validator/nym/directory/server/websocket"

	"github.com/gin-contrib/cors"
	swaggerFiles "github.com/swaggo/files"
	ginSwagger "github.com/swaggo/gin-swagger"
)

// New returns a new REST API server
func New() *gin.Engine {
	// Set the router as the default one shipped with Gin
	router := gin.Default()

	// Add cors middleware
	router.Use(cors.Default())

	// Serve Swagger frontend static files using gin-swagger middleware
	router.GET("/swagger/*any", ginSwagger.WrapHandler(swaggerFiles.Handler))

	// Add HTML templates to the router
	t, err := html.LoadTemplate()
	if err != nil {
		panic(err)
	}
	router.SetHTMLTemplate(t)
	router.GET("/", func(c *gin.Context) {
		c.HTML(http.StatusOK, "/server/html/index.html", nil)
	})

	// Set up websocket handlers
	hub := websocket.NewHub()
	go hub.Run()
	router.GET("/ws", func(c *gin.Context) {
		websocket.Serve(hub, c.Writer, c.Request)
	})

	// Sanitize controller input against XSS attacks using bluemonday.Policy
	policy := bluemonday.UGCPolicy()

	// Measurements: wire up dependency injection
	measurementsCfg := injectMeasurements(policy)

	// Presence: wire up dependency injection
	presenceCfg := injectPresence(policy)

	// Register all HTTP controller routes
	healthcheck.New().RegisterRoutes(router)
	mixmining.New(measurementsCfg).RegisterRoutes(router)
	presence.New(presenceCfg).RegisterRoutes(router)

	return router
}

func injectMeasurements(policy *bluemonday.Policy) mixmining.Config {
	sanitizer := mixmining.NewSanitizer(policy)
	db := mixmining.NewDb()
	mixminingService := *mixmining.NewService(db)

	return mixmining.Config{
		Service:   &mixminingService,
		Sanitizer: sanitizer,
	}
}


func injectPresence(policy *bluemonday.Policy) presence.Config {
	cocoSan := presence.NewCoconodeSanitizer(policy)
	mixSan := presence.NewMixnodeSanitizer(policy)
	mixnodeIDSan := presence.NewMixnodeIDSanitizer(policy)
	providerSan := presence.NewMixproviderSanitizer(policy)
	gatewaySan := presence.NewGatewaySanitizer(policy)
	presenceDb := presence.NewDb()
	service := presence.NewService(presenceDb)

	return presence.Config{
		CocoHostSanitizer:        &cocoSan,
		MixHostSanitizer:         &mixSan,
		MixNodeIDSanitizer:       &mixnodeIDSan,
		MixProviderHostSanitizer: &providerSan,
		GatewayHostSanitizer:     &gatewaySan,
		Service:                  service,
	}
}
