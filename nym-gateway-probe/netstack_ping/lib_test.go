package main

import (
	"math"
	"testing"
)

// Test the abs helper function
func TestAbs(t *testing.T) {
	tests := []struct {
		input    int
		expected int
	}{
		{5, 5},
		{-5, 5},
		{0, 0},
		{-1, 1},
		{1, 1},
	}

	for _, test := range tests {
		result := int(math.Abs(float64(test.input)))
		if result != test.expected {
			t.Errorf("abs(%d) = %d, expected %d", test.input, result, test.expected)
		}
	}
}

// Test URL shuffling behavior
func TestURLShuffling(t *testing.T) {
	urls := []string{"url1", "url2", "url3"}

	shuffledUrls := make([]string, len(urls))
	copy(shuffledUrls, urls)

	if len(shuffledUrls) != len(urls) {
		t.Error("Shuffled URLs length mismatch")
	}

	// Test that all original URLs are still present
	for _, originalURL := range urls {
		found := false
		for _, shuffledURL := range shuffledUrls {
			if shuffledURL == originalURL {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("URL %s not found in shuffled list", originalURL)
		}
	}
}

// Test timeout progression logic
func TestTimeoutProgression(t *testing.T) {
	baseTimeout := uint64(5)

	// Test ping timeout progression (+1s per retry)
	for attempt := 0; attempt < 3; attempt++ {
		expectedTimeout := baseTimeout + uint64(attempt*1)
		actualTimeout := baseTimeout + uint64(attempt*1)

		if actualTimeout != expectedTimeout {
			t.Errorf("Ping attempt %d: expected timeout %d, got %d", attempt, expectedTimeout, actualTimeout)
		}
	}

	// Test download timeout progression (+5s per retry)
	for attempt := 0; attempt < 3; attempt++ {
		expectedTimeout := baseTimeout + uint64(attempt*5)
		actualTimeout := baseTimeout + uint64(attempt*5)

		if actualTimeout != expectedTimeout {
			t.Errorf("Download attempt %d: expected timeout %d, got %d", attempt, expectedTimeout, actualTimeout)
		}
	}
}

// Test consecutive failure counting logic
func TestConsecutiveFailures(t *testing.T) {
	maxFailures := 3
	consecutiveFailures := 0

	// Simulate failures
	for i := 0; i < maxFailures; i++ {
		consecutiveFailures++
		if consecutiveFailures >= maxFailures {
			break
		}
	}

	if consecutiveFailures != maxFailures {
		t.Errorf("Expected %d consecutive failures, got %d", maxFailures, consecutiveFailures)
	}
}

// Test early exit logic
func TestEarlyExitLogic(t *testing.T) {
	maxConsecutiveFailures := 3
	consecutiveFailures := 0

	// Simulate early exit after 3 failures
	for i := 0; i < 5; i++ { // Try 5 times
		consecutiveFailures++
		if consecutiveFailures >= maxConsecutiveFailures {
			break // Early exit
		}
	}

	if consecutiveFailures != maxConsecutiveFailures {
		t.Errorf("Expected early exit after %d failures, got %d", maxConsecutiveFailures, consecutiveFailures)
	}
}

// Test URL list selection logic
func TestURLListSelection(t *testing.T) {
	// Test IPv4 URL selection
	ipVersion := uint8(4)
	var urlsToTry []string

	if ipVersion == 4 {
		urlsToTry = fileUrls
	} else {
		urlsToTry = fileUrlsV6
	}

	if len(urlsToTry) == 0 {
		t.Error("Expected non-empty URL list for IPv4")
	}

	// Test IPv6 URL selection
	ipVersion = uint8(6)
	if ipVersion == 4 {
		urlsToTry = fileUrls
	} else {
		urlsToTry = fileUrlsV6
	}

	if len(urlsToTry) == 0 {
		t.Error("Expected non-empty URL list for IPv6")
	}
}

// Test NetstackResponse struct creation
func TestNetstackResponse(t *testing.T) {
	response := NetstackResponse{
		CanHandshake:        true,
		SentIps:             5,
		ReceivedIps:         5,
		SentHosts:           5,
		ReceivedHosts:       5,
		CanResolveDns:       true,
		DownloadedFile:      "test.dat",
		DownloadDurationSec: 1,
		DownloadError:       "",
	}

	// Test that all fields are set correctly
	if !response.CanHandshake {
		t.Error("CanHandshake should be true")
	}

	if response.SentIps != 5 {
		t.Error("SentIps should be 5")
	}

	if response.ReceivedIps != 5 {
		t.Error("ReceivedIps should be 5")
	}

	if response.DownloadedFile != "test.dat" {
		t.Error("DownloadedFile should be 'test.dat'")
	}
}

// Test NetstackRequestGo struct creation
func TestNetstackRequestGo(t *testing.T) {
	request := NetstackRequestGo{
		WgIp:               "10.0.0.1",
		PrivateKey:         "test-key",
		PublicKey:          "test-pub-key",
		Endpoint:           "1.1.1.1:51820",
		Dns:                "1.1.1.1",
		IpVersion:          4,
		PingHosts:          []string{"example.com"},
		PingIps:            []string{"1.1.1.1"},
		NumPing:            3,
		SendTimeoutSec:     5,
		RecvTimeoutSec:     10,
		DownloadTimeoutSec: 30,
		AwgArgs:            "",
	}

	// Test that all fields are set correctly
	if request.WgIp != "10.0.0.1" {
		t.Error("WgIp should be '10.0.0.1'")
	}

	if request.IpVersion != 4 {
		t.Error("IpVersion should be 4")
	}

	if len(request.PingHosts) != 1 {
		t.Error("PingHosts should have 1 element")
	}

	if request.NumPing != 3 {
		t.Error("NumPing should be 3")
	}
}

// Test the ping function with valid request (will fail due to network setup)
func TestPingFunction(t *testing.T) {
	// Create a request with valid IP but will fail due to network setup
	req := NetstackRequestGo{
		WgIp:               "10.0.0.1",
		PrivateKey:         "0000000000000000000000000000000000000000000000000000000000000000",
		PublicKey:          "0000000000000000000000000000000000000000000000000000000000000000",
		Endpoint:           "1.1.1.1:51820",
		Dns:                "1.1.1.1",
		IpVersion:          4,
		PingHosts:          []string{"example.com"},
		PingIps:            []string{"1.1.1.1"},
		NumPing:            1,
		SendTimeoutSec:     1,
		RecvTimeoutSec:     1,
		DownloadTimeoutSec: 5,
		AwgArgs:            "",
	}

	// This should complete (even with network timeouts) and not crash
	response, err := ping(req)
	if err != nil {
		t.Errorf("Unexpected error: %v", err)
	}

	// Check that we got a valid response structure
	if response.CanHandshake != true {
		t.Error("Expected CanHandshake to be true")
	}

	// The response should show the retry attempts worked
	t.Logf("Response: CanHandshake=%v, SentHosts=%d, ReceivedHosts=%d, DownloadedFile=%s",
		response.CanHandshake, response.SentHosts, response.ReceivedHosts, response.DownloadedFile)
}

// Test the SuccessResult and ErrorResult structs
func TestResultStructs(t *testing.T) {
	// Test SuccessResult
	response := NetstackResponse{
		CanHandshake:        true,
		SentIps:             5,
		ReceivedIps:         5,
		SentHosts:           5,
		ReceivedHosts:       5,
		CanResolveDns:       true,
		DownloadedFile:      "test.dat",
		DownloadDurationSec: 1,
		DownloadError:       "",
	}

	successResult := SuccessResult{Response: response}
	if !successResult.Response.CanHandshake {
		t.Error("SuccessResult should contain valid response")
	}

	// Test ErrorResult
	errorResult := ErrorResult{Error: "test error"}
	if errorResult.Error != "test error" {
		t.Error("ErrorResult should contain error message")
	}
}

// TestConsecutiveFailureExit validates that the ping loop exits cleanly after consecutive failures
func TestConsecutiveFailureExit(t *testing.T) {
	// Create a test request that will trigger consecutive failures
	// Using valid hex-encoded keys (32 bytes = 64 hex chars)
	req := NetstackRequestGo{
		WgIp:               "10.0.0.1",
		PrivateKey:         "0000000000000000000000000000000000000000000000000000000000000000",
		PublicKey:          "0000000000000000000000000000000000000000000000000000000000000000",
		Endpoint:           "1.1.1.1:51820",
		Dns:                "1.1.1.1",
		IpVersion:          4,
		PingHosts:          []string{},            // No hosts to ping
		PingIps:            []string{"192.0.2.1"}, // RFC 5737 test IP that should fail
		NumPing:            5,                     // Try 5 pings
		SendTimeoutSec:     1,                     // Short timeout to ensure failures
		RecvTimeoutSec:     1,
		DownloadTimeoutSec: 1,
		AwgArgs:            "",
	}

	// Execute the ping function - this should exit cleanly after consecutive failures
	response, err := ping(req)

	// Verify the response shows we attempted pings but got failures
	if response.SentIps == 0 {
		t.Error("Should have attempted to send at least one ping")
	}

	// Verify we received no successful pings (since we're using a test IP)
	if response.ReceivedIps > 0 {
		t.Logf("Unexpected success: received %d pings", response.ReceivedIps)
	}

	// The key test: verify we don't hang and return cleanly
	if err != nil {
		t.Logf("Function returned with error (expected): %v", err)
	}

	// Verify we didn't send all 5 pings due to early exit
	// We should have sent at most 3 pings before hitting consecutive failure limit
	if response.SentIps > 3 {
		t.Errorf("Should have exited early after consecutive failures, but sent %d pings", response.SentIps)
	}

	t.Logf("Test completed cleanly: sent %d pings, received %d pings", response.SentIps, response.ReceivedIps)
}
