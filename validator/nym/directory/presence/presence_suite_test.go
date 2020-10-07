package presence_test

import (
	"testing"

	. "github.com/onsi/ginkgo"
	. "github.com/onsi/gomega"
)

func TestPresence(t *testing.T) {
	RegisterFailHandler(Fail)
	RunSpecs(t, "Presence Suite")
}
