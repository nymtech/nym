package mixmining

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"

	"github.com/nymtech/nym-directory/models"

	"github.com/gin-gonic/gin"
	"github.com/nymtech/nym-directory/mixmining/fixtures"
	"github.com/nymtech/nym-directory/mixmining/mocks"
	. "github.com/onsi/ginkgo"
	"github.com/stretchr/testify/assert"
)

var _ = Describe("Controller", func() {
	Describe("creating a mix status", func() {
		Context("from a host other than localhost", func() {
			It("should fail", func() {
				router, _, _ := SetupRouter()
				badJSON, _ := json.Marshal(fixtures.XSSMixStatus())
				resp := performNonLocalRequest(router, "POST", "/api/mixmining", badJSON)
				assert.Equal(GinkgoT(), 403, resp.Result().StatusCode)
			})
		})

		Context("that has 'false' set for 'Up'", func() {
			It("should save the mix status", func() {
				boolfalse := false
				router, mockService, mockSanitizer := SetupRouter()
				status := fixtures.GoodMixStatus()
				status.Up = &boolfalse

				savedStatus := fixtures.GoodPersistedMixStatus()
				savedStatus.Up = &boolfalse

				mockSanitizer.On("Sanitize", status).Return(status)
				mockService.On("CreateMixStatus", status).Return(savedStatus)
				mockService.On("SaveStatusReport", savedStatus).Return(models.MixStatusReport{})

				falseJSON, _ := json.Marshal(status)
				resp := performLocalHostRequest(router, "POST", "/api/mixmining", falseJSON)
				assert.Equal(GinkgoT(), 201, resp.Code)

			})
		})

		Context("containing xss", func() {
			It("should strip the xss attack, save the individual mix status, and update the status report for the given node", func() {
				router, mockService, mockSanitizer := SetupRouter()

				mockSanitizer.On("Sanitize", fixtures.XSSMixStatus()).Return(fixtures.GoodMixStatus())
				mockService.On("CreateMixStatus", fixtures.GoodMixStatus()).Return(fixtures.GoodPersistedMixStatus())
				mockService.On("SaveStatusReport", fixtures.GoodPersistedMixStatus()).Return(models.MixStatusReport{})
				badJSON, _ := json.Marshal(fixtures.XSSMixStatus())

				resp := performLocalHostRequest(router, "POST", "/api/mixmining", badJSON)
				var response map[string]string
				json.Unmarshal([]byte(resp.Body.String()), &response)

				assert.Equal(GinkgoT(), 201, resp.Code)
				mockSanitizer.AssertCalled(GinkgoT(), "Sanitize", fixtures.XSSMixStatus())
				mockService.AssertCalled(GinkgoT(), "CreateMixStatus", fixtures.GoodMixStatus())
			})
		})
	})

	Describe("retrieving a mix status report (overview)", func() {
		Context("when a report does not yet exist", func() {
			It("should 404", func() {
				router, mockService, _ := SetupRouter()
				mockService.On("GetStatusReport", fixtures.MixStatusReport().PubKey).Return(models.MixStatusReport{})
				resp := performLocalHostRequest(router, "GET", "/api/mixmining/key1/report", nil)
				assert.Equal(GinkgoT(), 404, resp.Result().StatusCode)
			})
		})

		Context("when a report exists", func() {
			It("should return the report", func() {
				router, mockService, _ := SetupRouter()
				mockService.On("GetStatusReport", fixtures.MixStatusReport().PubKey).Return(fixtures.MixStatusReport())
				resp := performLocalHostRequest(router, "GET", "/api/mixmining/key1/report", nil)
				var response models.MixStatusReport
				json.Unmarshal([]byte(resp.Body.String()), &response)
				assert.Equal(GinkgoT(), 200, resp.Result().StatusCode)
				assert.Equal(GinkgoT(), fixtures.MixStatusReport(), response)
			})
		})
	})

	Describe("listing statuses for a node", func() {
		Context("when no statuses have yet been saved", func() {
			It("returns an empty list", func() {
				router, mockService, _ := SetupRouter()
				mockService.On("List", "foo").Return([]models.PersistedMixStatus{})
				resp := performLocalHostRequest(router, "GET", "/api/mixmining/foo/history", nil)

				assert.Equal(GinkgoT(), 200, resp.Code)
			})
		})
		Context("when some statuses exist", func() {
			It("should return the list of statuses as json", func() {
				router, mockService, _ := SetupRouter()
				mockService.On("List", "pubkey1").Return(fixtures.MixStatusesList())
				url := "/api/mixmining/pubkey1/history"
				resp := performLocalHostRequest(router, "GET", url, nil)
				var response []models.PersistedMixStatus
				json.Unmarshal([]byte(resp.Body.String()), &response)

				assert.Equal(GinkgoT(), 200, resp.Code)
				assert.Equal(GinkgoT(), fixtures.MixStatusesList(), response)
			})
		})
	})
})

func SetupRouter() (*gin.Engine, *mocks.IService, *mocks.Sanitizer) {
	mockSanitizer := new(mocks.Sanitizer)
	mockService := new(mocks.IService)
	cfg := Config{
		Sanitizer: mockSanitizer,
		Service:   mockService,
	}
	router := gin.Default()
	controller := New(cfg)
	controller.RegisterRoutes(router)
	return router, mockService, mockSanitizer
}
func performLocalHostRequest(r http.Handler, method, path string, body []byte) *httptest.ResponseRecorder {
	buf := bytes.NewBuffer(body)
	req, _ := http.NewRequest(method, path, buf)
	req.RemoteAddr = "127.0.0.1"
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

func performNonLocalRequest(r http.Handler, method, path string, body []byte) *httptest.ResponseRecorder {
	buf := bytes.NewBuffer(body)
	req, _ := http.NewRequest(method, path, buf)
	req.RemoteAddr = "1.1.1.1"
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}
