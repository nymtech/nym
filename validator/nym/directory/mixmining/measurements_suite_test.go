package mixmining_test

import (
	"testing"

	. "github.com/onsi/ginkgo"
	. "github.com/onsi/gomega"
)

func TestMeasurements(t *testing.T) {
	RegisterFailHandler(Fail)
	RunSpecs(t, "Measurements Suite")
}
