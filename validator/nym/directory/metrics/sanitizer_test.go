package metrics

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
				result := sanitizer.Sanitize(xssMetric())
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

func xssMetric() models.MixMetric {
	sent := make(map[string]uint)
	sent["foo<script>alert('gotcha')</script>"] = 1
	received := uint(3)
	m := models.MixMetric{
		PubKey:   "bar<script>alert('gotcha')</script>",
		Sent:     sent,
		Received: &received,
	}
	return m
}

func goodMetric() models.MixMetric {
	sent := make(map[string]uint)
	sent["foo"] = 1
	received := uint(3)
	m := models.MixMetric{
		PubKey:   "bar",
		Sent:     sent,
		Received: &received,
	}
	return m
}
