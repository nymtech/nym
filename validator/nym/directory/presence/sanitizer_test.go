package presence

import (
	"github.com/microcosm-cc/bluemonday"
	"github.com/nymtech/nym-directory/presence/fixtures"
	. "github.com/onsi/ginkgo"
	"github.com/stretchr/testify/assert"
)

var _ = Describe("Sanitizer", func() {
	Describe("sanitizing inputs", func() {
		Context("for CocoHostInfo", func() {
			Context("when XSS is present", func() {
				It("sanitizes input", func() {
					policy := bluemonday.UGCPolicy()
					sanitizer := NewCoconodeSanitizer(policy)

					result := sanitizer.Sanitize(fixtures.XssCocoHost())
					assert.Equal(GinkgoT(), fixtures.GoodCocoHost(), result)
				})
			})
			Context("when XSS is not present", func() {
				It("doesn't change input", func() {
					policy := bluemonday.UGCPolicy()
					sanitizer := NewCoconodeSanitizer(policy)
					result := sanitizer.Sanitize(fixtures.GoodCocoHost())
					assert.Equal(GinkgoT(), fixtures.GoodCocoHost(), result)
				})
			})
		})
	})
	Context("for MixHostInfo", func() {
		Context("when XSS is present", func() {
			It("sanitizes input", func() {
				policy := bluemonday.UGCPolicy()
				sanitizer := NewMixnodeSanitizer(policy)

				result := sanitizer.Sanitize(fixtures.XssMixHost())
				assert.Equal(GinkgoT(), fixtures.GoodMixHost(), result)
			})
		})
		Context("when XSS is not present", func() {
			It("doesn't change input", func() {
				policy := bluemonday.UGCPolicy()
				sanitizer := NewMixnodeSanitizer(policy)
				result := sanitizer.Sanitize(fixtures.GoodMixHost())
				assert.Equal(GinkgoT(), fixtures.GoodMixHost(), result)
			})
		})
	})
	Context("for GatewayHostInfo", func() {
		Context("when XSS is present", func() {
			It("sanitizes input", func() {
				policy := bluemonday.UGCPolicy()
				sanitizer := NewGatewaySanitizer(policy)

				result := sanitizer.Sanitize(fixtures.XssGatewayHost())
				assert.Equal(GinkgoT(), fixtures.GoodGatewayHost(), result)
			})
		})
		Context("when XSS is not present", func() {
			It("doesn't change input", func() {
				policy := bluemonday.UGCPolicy()
				sanitizer := NewGatewaySanitizer(policy)
				result := sanitizer.Sanitize(fixtures.GoodGatewayHost())
				assert.Equal(GinkgoT(), fixtures.GoodGatewayHost(), result)
			})
		})
	})
})
