package mixmining

import (
	"github.com/microcosm-cc/bluemonday"
	"github.com/nymtech/nym-directory/models"
	. "github.com/onsi/ginkgo"
	"github.com/stretchr/testify/assert"
)

var _ = Describe("Sanitizer", func() {
	Describe("sanitizing inputs", func() {
		Context("when XSS is present", func() {
			It("sanitizes input", func() {
				policy := bluemonday.UGCPolicy()
				sanitizer := NewSanitizer(policy)
				result := sanitizer.Sanitize(xssStatus())
				assert.Equal(GinkgoT(), goodMetric(), result)
			})
		})
		Context("when XSS is not present", func() {
			It("doesn't change input", func() {
				policy := bluemonday.UGCPolicy()
				sanitizer := NewSanitizer(policy)
				result := sanitizer.Sanitize(goodMetric())
				assert.Equal(GinkgoT(), goodMetric(), result)
			})
		})
	})
})

func xssStatus() models.MixStatus {
	boolfalse := false
	m := models.MixStatus{
		PubKey:    "bar<script>alert('gotcha')</script>",
		Up:        &boolfalse,
		IPVersion: "0<script>alert('gotcha')</script>",
	}
	return m
}

func goodMetric() models.MixStatus {
	boolfalse := false
	m := models.MixStatus{
		PubKey:    "bar",
		Up:        &boolfalse,
		IPVersion: "0",
	}
	return m
}
