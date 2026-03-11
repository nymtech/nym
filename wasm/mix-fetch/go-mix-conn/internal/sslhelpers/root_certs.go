package sslhelpers

import (
	"crypto/x509"
	_ "embed"
	"go-mix-conn/internal/log"
	"strings"
	"sync"
)

// cacert.pem is the Mozilla CA root certificate bundle from curl.se.
// To update, run: scripts/update-root-certs.sh
//
//go:embed cacert.pem
var caCertPEM []byte

var (
	rootCertPool *x509.CertPool
	rootCertOnce sync.Once
)

func rootCerts() *x509.CertPool {
	rootCertOnce.Do(func() {
		// Log the bundle date from the PEM header once at first use.
		for _, line := range strings.SplitN(string(caCertPEM), "\n", 20) {
			if strings.Contains(line, "Certificate data from Mozilla as of:") {
				log.Info("Root CA bundle: %s", strings.TrimPrefix(line, "## "))
				break
			}
		}

		rootCertPool = x509.NewCertPool()
		ok := rootCertPool.AppendCertsFromPEM(caCertPEM)
		if !ok {
			log.Error("failed to parse any certificates from embedded cacert.pem")
			panic("failed to parse root certificates")
		}
	})

	return rootCertPool
}
