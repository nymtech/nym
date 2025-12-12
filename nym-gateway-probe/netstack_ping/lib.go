/* SPDX-License-Identifier: MIT
 *
 * Copyright (C) 2024 Nym Technologies SA <contact@nymtech.net>. All Rights Reserved.
 */

package main

// #include <stdlib.h>
import "C"

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"math"
	"math/rand"
	"net"
	"net/http"
	"net/netip"
	netUrl "net/url"
	"os"
	"strings"
	"time"
	"unsafe"

	"github.com/amnezia-vpn/amneziawg-go/conn"
	"github.com/amnezia-vpn/amneziawg-go/device"
	"github.com/amnezia-vpn/amneziawg-go/tun/netstack"
	"golang.org/x/net/icmp"
	"golang.org/x/net/ipv4"
	"golang.org/x/net/ipv6"
)

var fileUrls = []string{
	"https://proof.ovh.net/files/10Mb.dat",
	"https://nym-bandwidth-monitoring.ops-d86.workers.dev/10mb.dat",
	// "https://nym-bandwidth-monitoring.ops-d86.workers.dev/100mb.dat", to be introduced later
}

var fileUrlsV6 = []string{
	"https://proof.ovh.net/files/10Mb.dat",
	"https://nym-bandwidth-monitoring.ops-d86.workers.dev/10mb.dat",
	// "https://nym-bandwidth-monitoring.ops-d86.workers.dev/100mb.dat", to be introduced later
}

type NetstackRequestGo struct {
	WgIp               string   `json:"wg_ip"`
	PrivateKey         string   `json:"private_key"`
	PublicKey          string   `json:"public_key"`
	Endpoint           string   `json:"endpoint"`
	MetadataEndpoint   string   `json:"metadata_endpoint"`
	Dns                string   `json:"dns"`
	IpVersion          uint8    `json:"ip_version"`
	PingHosts          []string `json:"ping_hosts"`
	PingIps            []string `json:"ping_ips"`
	NumPing            uint8    `json:"num_ping"`
	SendTimeoutSec     uint64   `json:"send_timeout_sec"`
	RecvTimeoutSec     uint64   `json:"recv_timeout_sec"`
	DownloadTimeoutSec uint64   `json:"download_timeout_sec"`
	MetadataTimeoutSec uint64   `json:"metadata_timeout_sec"`
	AwgArgs            string   `json:"awg_args"`
}

type NetstackResponse struct {
	CanHandshake                 bool   `json:"can_handshake"`
	CanQueryMetadata             bool   `json:"can_query_metadata"`
	SentIps                      uint16 `json:"sent_ips"`
	ReceivedIps                  uint16 `json:"received_ips"`
	SentHosts                    uint16 `json:"sent_hosts"`
	ReceivedHosts                uint16 `json:"received_hosts"`
	CanResolveDns                bool   `json:"can_resolve_dns"`
	DownloadedFile               string `json:"downloaded_file"`
	DownloadedFileSizeBytes      uint64 `json:"downloaded_file_size_bytes"`
	DownloadDurationSec          uint64 `json:"download_duration_sec"`
	DownloadDurationMilliseconds uint64 `json:"download_duration_milliseconds"`
	DownloadError                string `json:"download_error"`
}

type SuccessResult = struct {
	Response NetstackResponse `json:"response"`
}

type ErrorResult = struct {
	Error string `json:"error"`
}

func jsonResponse(response NetstackResponse) *C.char {
	bytes, serializeErr := json.Marshal(SuccessResult{
		Response: response,
	})
	if serializeErr == nil {
		return C.CString(string(bytes))
	} else {
		return C.CString("{\"error\":\"" + serializeErr.Error() + "\"}")
	}
}

func jsonError(err error) *C.char {
	jsonErr := ErrorResult{
		Error: fmt.Sprintf("failed to parse request: %s", err.Error()),
	}
	bytes, serializeErr := json.Marshal(jsonErr)
	if serializeErr == nil {
		return C.CString(string(bytes))
	} else {
		return C.CString("{\"error\":\"" + serializeErr.Error() + "\"}")
	}
}

//export wgPing
func wgPing(cReq *C.char) *C.char {
	reqStr := C.GoString(cReq)

	var req NetstackRequestGo
	err := json.Unmarshal([]byte(reqStr), &req)
	if err != nil {
		log.Printf("Failed to parse request: %s", err)
		return jsonError(err)
	}

	response, err := ping(req)
	if err != nil {
		log.Printf("Failed to ping: %s", err)
		return jsonError(err)
	}

	return jsonResponse(response)
}

//export wgFreePtr
func wgFreePtr(ptr unsafe.Pointer) {
	C.free(ptr)
}

// TwoHopNetstackRequest contains configuration for two-hop WireGuard tunneling.
// Traffic flows: Client -> Entry WG Tunnel -> UDP Forwarder -> Exit WG Tunnel -> Internet
type TwoHopNetstackRequest struct {
	// Entry tunnel configuration (connects to entry gateway)
	EntryWgIp       string `json:"entry_wg_ip"`
	EntryPrivateKey string `json:"entry_private_key"`
	EntryPublicKey  string `json:"entry_public_key"`
	EntryEndpoint   string `json:"entry_endpoint"`
	EntryAwgArgs    string `json:"entry_awg_args"`

	// Exit tunnel configuration (connects via forwarder through entry)
	ExitWgIp       string `json:"exit_wg_ip"`
	ExitPrivateKey string `json:"exit_private_key"`
	ExitPublicKey  string `json:"exit_public_key"`
	ExitEndpoint   string `json:"exit_endpoint"` // Actual exit gateway endpoint (forwarded via entry)
	ExitAwgArgs    string `json:"exit_awg_args"`

	// Test parameters (same as single-hop)
	Dns                string   `json:"dns"`
	IpVersion          uint8    `json:"ip_version"`
	PingHosts          []string `json:"ping_hosts"`
	PingIps            []string `json:"ping_ips"`
	NumPing            uint8    `json:"num_ping"`
	SendTimeoutSec     uint64   `json:"send_timeout_sec"`
	RecvTimeoutSec     uint64   `json:"recv_timeout_sec"`
	DownloadTimeoutSec uint64   `json:"download_timeout_sec"`
}

// Default port that exit WG tunnel uses to send traffic to the forwarder.
// The forwarder only accepts packets from this port on loopback.
const DEFAULT_EXIT_WG_CLIENT_PORT uint16 = 54001

// Entry tunnel MTU (outer tunnel)
const ENTRY_MTU = 1420

// Exit tunnel MTU (must be smaller due to double encapsulation)
const EXIT_MTU = 1340

//export wgPingTwoHop
func wgPingTwoHop(cReq *C.char) *C.char {
	reqStr := C.GoString(cReq)

	var req TwoHopNetstackRequest
	err := json.Unmarshal([]byte(reqStr), &req)
	if err != nil {
		log.Printf("Failed to parse two-hop request: %s", err)
		return jsonError(err)
	}

	response, err := pingTwoHop(req)
	if err != nil {
		log.Printf("Failed to ping (two-hop): %s", err)
		return jsonError(err)
	}

	return jsonResponse(response)
}

func pingTwoHop(req TwoHopNetstackRequest) (NetstackResponse, error) {
	log.Printf("=== Two-Hop WireGuard Probe ===")
	log.Printf("Entry endpoint: %s", req.EntryEndpoint)
	log.Printf("Entry WG IP: %s", req.EntryWgIp)
	log.Printf("Exit endpoint: %s (via entry forwarding)", req.ExitEndpoint)
	log.Printf("Exit WG IP: %s", req.ExitWgIp)
	log.Printf("IP version: %d", req.IpVersion)

	response := NetstackResponse{false, false, 0, 0, 0, 0, false, "", 0, 0, 0, ""}

	// Parse the exit endpoint to determine IP version for forwarder
	exitEndpoint, err := netip.ParseAddrPort(req.ExitEndpoint)
	if err != nil {
		return response, fmt.Errorf("failed to parse exit endpoint: %w", err)
	}

	// ============================================
	// STEP 1: Create entry tunnel (netstack)
	// ============================================
	log.Printf("Creating entry tunnel (MTU=%d)...", ENTRY_MTU)

	entryTun, entryTnet, err := netstack.CreateNetTUN(
		[]netip.Addr{netip.MustParseAddr(req.EntryWgIp)},
		[]netip.Addr{netip.MustParseAddr(req.Dns)},
		ENTRY_MTU)
	if err != nil {
		return response, fmt.Errorf("failed to create entry tunnel: %w", err)
	}

	entryLogger := device.NewLogger(device.LogLevelError, "entry: ")
	entryDev := device.NewDevice(entryTun, conn.NewDefaultBind(), entryLogger)
	defer entryDev.Close()

	// Configure entry device
	var entryIpc strings.Builder
	entryIpc.WriteString("private_key=")
	entryIpc.WriteString(req.EntryPrivateKey)
	if req.EntryAwgArgs != "" {
		awg := strings.ReplaceAll(req.EntryAwgArgs, "\\n", "\n")
		entryIpc.WriteString(fmt.Sprintf("\n%s", awg))
	}
	entryIpc.WriteString("\npublic_key=")
	entryIpc.WriteString(req.EntryPublicKey)
	entryIpc.WriteString("\nendpoint=")
	entryIpc.WriteString(req.EntryEndpoint)
	// Entry tunnel routes all traffic (the exit endpoint IP goes through it)
	entryIpc.WriteString("\nallowed_ip=0.0.0.0/0")
	entryIpc.WriteString("\nallowed_ip=::/0\n")

	if err := entryDev.IpcSet(entryIpc.String()); err != nil {
		return response, fmt.Errorf("failed to configure entry device: %w", err)
	}

	if err := entryDev.Up(); err != nil {
		return response, fmt.Errorf("failed to bring up entry device: %w", err)
	}
	log.Printf("Entry tunnel up")

	// ============================================
	// STEP 2: Create UDP forwarder
	// ============================================
	log.Printf("Creating UDP forwarder (exit endpoint: %s)...", exitEndpoint.String())

	forwarderConfig := UDPForwarderConfig{
		ListenPort: 0, // Dynamic port assignment
		ClientPort: DEFAULT_EXIT_WG_CLIENT_PORT,
		Endpoint:   exitEndpoint,
	}

	forwarder, err := NewUDPForwarder(forwarderConfig, entryTnet, entryLogger)
	if err != nil {
		return response, fmt.Errorf("failed to create UDP forwarder: %w", err)
	}
	defer forwarder.Close()

	forwarderAddr := forwarder.GetListenAddr()
	log.Printf("UDP forwarder listening on: %s", forwarderAddr.String())

	// ============================================
	// STEP 3: Create exit tunnel (netstack)
	// ============================================
	log.Printf("Creating exit tunnel (MTU=%d)...", EXIT_MTU)

	exitTun, exitTnet, err := netstack.CreateNetTUN(
		[]netip.Addr{netip.MustParseAddr(req.ExitWgIp)},
		[]netip.Addr{netip.MustParseAddr(req.Dns)},
		EXIT_MTU)
	if err != nil {
		return response, fmt.Errorf("failed to create exit tunnel: %w", err)
	}

	exitLogger := device.NewLogger(device.LogLevelError, "exit: ")
	exitDev := device.NewDevice(exitTun, conn.NewDefaultBind(), exitLogger)
	defer exitDev.Close()

	// Configure exit device - endpoint is the forwarder, NOT the actual exit gateway
	var exitIpc strings.Builder
	exitIpc.WriteString("private_key=")
	exitIpc.WriteString(req.ExitPrivateKey)
	// Set listen_port so the forwarder knows which port to accept packets from
	exitIpc.WriteString(fmt.Sprintf("\nlisten_port=%d", DEFAULT_EXIT_WG_CLIENT_PORT))
	if req.ExitAwgArgs != "" {
		awg := strings.ReplaceAll(req.ExitAwgArgs, "\\n", "\n")
		exitIpc.WriteString(fmt.Sprintf("\n%s", awg))
	}
	exitIpc.WriteString("\npublic_key=")
	exitIpc.WriteString(req.ExitPublicKey)
	// IMPORTANT: endpoint is the local forwarder, not the actual exit gateway!
	exitIpc.WriteString("\nendpoint=")
	exitIpc.WriteString(forwarderAddr.String())
	if req.IpVersion == 4 {
		exitIpc.WriteString("\nallowed_ip=0.0.0.0/0\n")
	} else {
		exitIpc.WriteString("\nallowed_ip=::/0\n")
	}

	if err := exitDev.IpcSet(exitIpc.String()); err != nil {
		return response, fmt.Errorf("failed to configure exit device: %w", err)
	}

	if err := exitDev.Up(); err != nil {
		return response, fmt.Errorf("failed to bring up exit device: %w", err)
	}
	log.Printf("Exit tunnel up (via forwarder)")

	// If we got here, both tunnels and forwarder are set up
	response.CanHandshake = true
	log.Printf("Two-hop tunnel setup complete!")

	// ============================================
	// STEP 4: Run tests through exit tunnel
	// ============================================
	log.Printf("Running tests through exit tunnel...")

	// Ping hosts (DNS resolution test)
	for _, host := range req.PingHosts {
		consecutiveFailures := 0
		maxConsecutiveFailures := 3

		for i := uint8(0); i < req.NumPing; i++ {
			log.Printf("Pinging %s seq=%d (via two-hop)", host, i)
			response.SentHosts += 1
			rt, err := sendPing(host, i, req.SendTimeoutSec, req.RecvTimeoutSec, exitTnet, req.IpVersion)
			if err != nil {
				log.Printf("Failed to send ping: %v", err)
				consecutiveFailures++
				if consecutiveFailures >= maxConsecutiveFailures {
					log.Printf("Too many consecutive failures (%d), stopping ping attempts for %s", consecutiveFailures, host)
					break
				}
				continue
			}
			consecutiveFailures = 0
			response.ReceivedHosts += 1
			response.CanResolveDns = true
			log.Printf("Ping latency: %v", rt)
		}
	}

	// Ping IPs (direct connectivity test)
	for _, ip := range req.PingIps {
		consecutiveFailures := 0
		maxConsecutiveFailures := 3

		for i := uint8(0); i < req.NumPing; i++ {
			log.Printf("Pinging %s seq=%d (via two-hop)", ip, i)
			response.SentIps += 1
			rt, err := sendPing(ip, i, req.SendTimeoutSec, req.RecvTimeoutSec, exitTnet, req.IpVersion)
			if err != nil {
				log.Printf("Failed to send ping: %v", err)
				consecutiveFailures++
				if consecutiveFailures >= maxConsecutiveFailures {
					log.Printf("Too many consecutive failures (%d), stopping ping attempts for %s", consecutiveFailures, ip)
					break
				}
			} else {
				consecutiveFailures = 0
				response.ReceivedIps += 1
				log.Printf("Ping latency: %v", rt)
			}

			if i < req.NumPing-1 {
				time.Sleep(5 * time.Second)
			}
		}
	}

	// Download test
	var urlsToTry []string
	if req.IpVersion == 4 {
		urlsToTry = fileUrls
	} else {
		urlsToTry = fileUrlsV6
	}

	fileContent, downloadDuration, usedURL, err := downloadFileWithRetry(urlsToTry, req.DownloadTimeoutSec, exitTnet)
	if err != nil {
		log.Printf("Failed to download file from any URL: %v", err)
		response.DownloadError = err.Error()
	} else {
		log.Printf("Downloaded file content length: %.2f MB", float64(len(fileContent))/1024/1024)
		log.Printf("Download duration: %v", downloadDuration)
		response.DownloadedFileSizeBytes = uint64(len(fileContent))
	}

	response.DownloadDurationSec = uint64(downloadDuration.Seconds())
	response.DownloadDurationMilliseconds = uint64(downloadDuration.Milliseconds())
	response.DownloadedFile = usedURL

	log.Printf("=== Two-Hop Probe Complete ===")
	return response, nil
}

func ping(req NetstackRequestGo) (NetstackResponse, error) {
	fmt.Printf("Endpoint: %s\n", req.Endpoint)
	fmt.Printf("WireGuard IP: %s\n", req.WgIp)
	fmt.Printf("IP version: %d\n", req.IpVersion)

	tun, tnet, err := netstack.CreateNetTUN(
		[]netip.Addr{netip.MustParseAddr(req.WgIp)},
		[]netip.Addr{netip.MustParseAddr(req.Dns)},
		1280)

	if err != nil {
		return NetstackResponse{}, err
	}
	dev := device.NewDevice(tun, conn.NewDefaultBind(), device.NewLogger(device.LogLevelError, ""))

	var ipc strings.Builder

	ipc.WriteString("private_key=")
	ipc.WriteString(req.PrivateKey)
	if req.AwgArgs != "" {
		awg := strings.ReplaceAll(req.AwgArgs, "\\n", "\n")
		ipc.WriteString(fmt.Sprintf("\n%s", awg))
	}
	ipc.WriteString("\npublic_key=")
	ipc.WriteString(req.PublicKey)
	ipc.WriteString("\nendpoint=")
	ipc.WriteString(req.Endpoint)
	if req.IpVersion == 4 {
		ipc.WriteString("\nallowed_ip=0.0.0.0/0\n")
	} else {
		ipc.WriteString("\nallowed_ip=::/0\n")
	}

	response := NetstackResponse{false, false, 0, 0, 0, 0, false, "", 0, 0, 0, ""}

	err = dev.IpcSet(ipc.String())
	if err != nil {
		return NetstackResponse{}, err
	}

	config, err := dev.IpcGet()
	if err != nil {
		return NetstackResponse{}, err
	}

	// do not print the config by default, because it contains the wg private key
	if os.Getenv("SHOW_WG_CONFIG") == "true" {
		log.Printf("%s", config)
	}

	err = dev.Up()
	if err != nil {
		return NetstackResponse{}, err
	}

	response.CanHandshake = true

	// Skip metadata query if endpoint is empty (e.g., for IPv6 where the IPv4 metadata endpoint is not reachable)
	if req.MetadataEndpoint != "" {
		version, duration, err := queryMetadata(req.MetadataEndpoint, req.MetadataTimeoutSec, tnet)
		if err != nil {
			log.Printf("Failed to query metadata URLs: %v\n", err)
			response.CanQueryMetadata = false
		} else {
			log.Printf("Queried metadata endpoint with version: %v\n", version)
			log.Printf("Query duration: %v\n", duration)
			response.CanQueryMetadata = true
		}
	} else {
		log.Printf("Skipping metadata query (no endpoint provided)")
		response.CanQueryMetadata = false
	}

	for _, host := range req.PingHosts {
		consecutiveFailures := 0
		maxConsecutiveFailures := 3

		for i := uint8(0); i < req.NumPing; i++ {
			log.Printf("Pinging %s seq=%d", host, i)
			response.SentHosts += 1
			rt, err := sendPing(host, i, req.SendTimeoutSec, req.RecvTimeoutSec, tnet, req.IpVersion)
			if err != nil {
				log.Printf("Failed to send ping: %v\n", err)
				consecutiveFailures++

				// Early exit if too many consecutive failures
				if consecutiveFailures >= maxConsecutiveFailures {
					log.Printf("Too many consecutive failures (%d), stopping ping attempts for %s", consecutiveFailures, host)
					break
				}
				continue
			}

			// Reset failure counter on success
			consecutiveFailures = 0
			response.ReceivedHosts += 1
			response.CanResolveDns = true
			log.Printf("Ping latency: %v\n", rt)
		}
	}

	for _, ip := range req.PingIps {
		consecutiveFailures := 0
		maxConsecutiveFailures := 3

		for i := uint8(0); i < req.NumPing; i++ {
			log.Printf("Pinging %s seq=%d", ip, i)
			response.SentIps += 1
			rt, err := sendPing(ip, i, req.SendTimeoutSec, req.RecvTimeoutSec, tnet, req.IpVersion)
			if err != nil {
				log.Printf("Failed to send ping: %v\n", err)
				consecutiveFailures++

				// Early exit if too many consecutive failures
				if consecutiveFailures >= maxConsecutiveFailures {
					log.Printf("Too many consecutive failures (%d), stopping ping attempts for %s", consecutiveFailures, ip)
					break
				}
			} else {
				// Reset failure counter on success
				consecutiveFailures = 0
				response.ReceivedIps += 1
				log.Printf("Ping latency: %v\n", rt)
			}

			// Sleep between ping attempts (except for the last one)
			if i < req.NumPing-1 {
				time.Sleep(5 * time.Second)
			}
		}
	}

	var urlsToTry []string

	if req.IpVersion == 4 {
		urlsToTry = fileUrls
	} else {
		urlsToTry = fileUrlsV6
	}

	// Try URLs with retry logic
	fileContent, downloadDuration, usedURL, err := downloadFileWithRetry(urlsToTry, req.DownloadTimeoutSec, tnet)
	if err != nil {
		log.Printf("Failed to download file from any URL: %v\n", err)
	} else {
		log.Printf("Downloaded file content length: %.2f MB\n", float64(len(fileContent))/1024/1024)
		log.Printf("Download duration: %v\n", downloadDuration)
	}

	response.DownloadDurationSec = uint64(downloadDuration.Seconds())
	response.DownloadDurationMilliseconds = uint64(downloadDuration.Milliseconds())
	response.DownloadedFile = usedURL
	if err != nil {
		response.DownloadError = err.Error()
		response.DownloadedFileSizeBytes = 0
	} else {
		response.DownloadError = ""
		response.DownloadedFileSizeBytes = uint64(len(fileContent))
	}

	return response, nil
}

func sendPing(address string, seq uint8, sendTtimeoutSecs uint64, receiveTimoutSecs uint64, tnet *netstack.Net, ipVersion uint8) (time.Duration, error) {
	maxPingRetries := 2
	baseTimeout := receiveTimoutSecs

	for attempt := 0; attempt < maxPingRetries; attempt++ {
		// Slightly increase timeout on retries, but keep it reasonable
		adjustedTimeout := baseTimeout + uint64(attempt*1) // +1s per retry only

		duration, err := sendPingAttempt(address, seq, sendTtimeoutSecs, adjustedTimeout, tnet, ipVersion)
		if err == nil {
			return duration, nil
		}

		log.Printf("Ping attempt %d/%d failed: %v", attempt+1, maxPingRetries, err)
		if attempt < maxPingRetries-1 {
			time.Sleep(200 * time.Millisecond) // Very brief delay between retries
		}
	}

	return 0, fmt.Errorf("ping failed after %d attempts", maxPingRetries)
}

func sendPingAttempt(address string, seq uint8, sendTtimeoutSecs uint64, receiveTimoutSecs uint64, tnet *netstack.Net, ipVersion uint8) (time.Duration, error) {
	var socket net.Conn
	var err error
	if ipVersion == 4 {
		socket, err = tnet.Dial("ping4", address)
	} else {
		socket, err = tnet.Dial("ping6", address)
	}

	if err != nil {
		return 0, err
	}
	defer socket.Close()

	var icmpBytes []byte

	requestPing := icmp.Echo{
		ID:   1337,
		Seq:  int(seq),
		Data: []byte("gopher burrow"),
	}

	if ipVersion == 4 {
		icmpBytes, _ = (&icmp.Message{Type: ipv4.ICMPTypeEcho, Code: 0, Body: &requestPing}).Marshal(nil)
	} else {
		icmpBytes, _ = (&icmp.Message{Type: ipv6.ICMPTypeEchoRequest, Code: 0, Body: &requestPing}).Marshal(nil)
	}

	start := time.Now()

	socket.SetWriteDeadline(time.Now().Add(time.Second * time.Duration(sendTtimeoutSecs)))
	_, err = socket.Write(icmpBytes)
	if err != nil {
		return 0, err
	}

	// Wait for reply with limited read attempts to avoid long delays
	maxReadAttempts := 2
	for readAttempt := 0; readAttempt < maxReadAttempts; readAttempt++ {
		socket.SetReadDeadline(time.Now().Add(time.Second * time.Duration(receiveTimoutSecs)))
		n, err := socket.Read(icmpBytes[:])
		if err != nil {
			if readAttempt < maxReadAttempts-1 {
				log.Printf("Read attempt %d failed, retrying: %v", readAttempt+1, err)
				continue
			}
			return 0, err
		}

		var proto int
		if ipVersion == 4 {
			proto = 1
		} else {
			proto = 58
		}

		replyPacket, err := icmp.ParseMessage(proto, icmpBytes[:n])
		if err != nil {
			if readAttempt < maxReadAttempts-1 {
				log.Printf("Parse attempt %d failed, retrying: %v", readAttempt+1, err)
				continue
			}
			return 0, err
		}

		var ok bool
		replyPing, ok := replyPacket.Body.(*icmp.Echo)

		if !ok {
			if readAttempt < maxReadAttempts-1 {
				log.Printf("Invalid reply type attempt %d, retrying", readAttempt+1)
				continue
			}
			return 0, fmt.Errorf("invalid reply type: %v", replyPacket)
		}

		if bytes.Equal(replyPing.Data, requestPing.Data) {
			// Accept sequence number matches or close matches (for out-of-order delivery)
			if replyPing.Seq == requestPing.Seq || math.Abs(float64(replyPing.Seq-requestPing.Seq)) <= 1 {
				return time.Since(start), nil
			}
			log.Printf("Sequence mismatch (expected %d, received %d), retrying", requestPing.Seq, replyPing.Seq)
		} else {
			if readAttempt < maxReadAttempts-1 {
				log.Printf("Data mismatch attempt %d, retrying", readAttempt+1)
				continue
			}
			return 0, fmt.Errorf("invalid ping reply: %v (request: %v)", replyPing, requestPing)
		}
	}

	return 0, fmt.Errorf("ping failed after %d read attempts", maxReadAttempts)
}

func downloadFileWithRetry(urls []string, timeoutSecs uint64, tnet *netstack.Net) ([]byte, time.Duration, string, error) {
	maxRetries := 3
	baseDelay := 1 * time.Second
	consecutiveFailures := 0
	maxConsecutiveFailures := 3

	for attempt := 0; attempt < maxRetries; attempt++ {
		// Shuffle URLs for each attempt to try different ones
		shuffledUrls := make([]string, len(urls))
		copy(shuffledUrls, urls)
		rand.Shuffle(len(shuffledUrls), func(i, j int) {
			shuffledUrls[i], shuffledUrls[j] = shuffledUrls[j], shuffledUrls[i]
		})

		for _, url := range shuffledUrls {
			log.Printf("Attempting download from: %s (attempt %d/%d)", url, attempt+1, maxRetries)
			// Increase timeout on retries to handle slow servers
			adjustedTimeout := timeoutSecs + uint64(attempt*5) // +5s per retry
			content, duration, err := downloadFile(url, adjustedTimeout, tnet)
			if err == nil {
				log.Printf("Successfully downloaded from: %s", url)
				return content, duration, url, nil
			}
			log.Printf("Failed to download from %s: %v", url, err)
			consecutiveFailures++

			// Early exit if too many consecutive failures
			if consecutiveFailures >= maxConsecutiveFailures {
				log.Printf("Too many consecutive download failures (%d), stopping attempts", consecutiveFailures)
				return nil, 0, "", fmt.Errorf("too many consecutive failures (%d), stopping download attempts", consecutiveFailures)
			}
		}

		if attempt < maxRetries-1 {
			delay := baseDelay * time.Duration(attempt+1)
			log.Printf("All URLs failed, retrying in %v...", delay)
			time.Sleep(delay)
		}
	}

	return nil, 0, "", fmt.Errorf("failed to download from any URL after %d attempts", maxRetries)
}

func downloadFile(url string, timeoutSecs uint64, tnet *netstack.Net) ([]byte, time.Duration, error) {
	transport := &http.Transport{
		DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
			return tnet.Dial(network, addr)
		},
	}

	client := &http.Client{
		Transport: transport,
		Timeout:   time.Second * time.Duration(timeoutSecs),
	}

	start := time.Now() // Start timing

	resp, err := client.Get(url)
	if err != nil {
		return nil, 0, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, 0, fmt.Errorf("failed to download file: %s", resp.Status)
	}

	var buf bytes.Buffer
	_, err = io.Copy(&buf, resp.Body)
	if err != nil {
		return nil, 0, err
	}

	duration := time.Since(start) // Calculate duration

	return buf.Bytes(), duration, nil
}

func queryMetadata(url string, timeoutSecs uint64, tnet *netstack.Net) (int, time.Duration, error) {
	transport := &http.Transport{
		DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
			return tnet.Dial(network, addr)
		},
	}

	client := &http.Client{
		Transport: transport,
		Timeout:   time.Second * time.Duration(timeoutSecs),
	}

	bandwidthVersionUrl, err := netUrl.JoinPath(url, "v1/bandwidth/version")
	if err != nil {
		return 0, 0, err
	}

	start := time.Now() // Start timing

	log.Printf("Querying metadata encoding: url = %s", bandwidthVersionUrl)
	resp, err := client.Get(bandwidthVersionUrl)
	if err != nil {
		return 0, 0, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return 0, 0, fmt.Errorf("failed to query metadata endpoint: %s", resp.Status)
	}

	var contentType = resp.Header.Get("Content-Type")

	log.Printf("Metadata Content-Type: %s", contentType)

	var reader io.Reader = resp.Body
	bodyBytes, err := io.ReadAll(reader)
	if err != nil {
		return 0, 0, err
	}

	var version int
	err = json.Unmarshal(bodyBytes, &version)
	if err != nil {
		return 0, 0, err
	}

	duration := time.Since(start) // Calculate duration

	return version, duration, nil
}

func main() {
	// uncomment the lines below to run locally and see README.md for how to get the Wireguard config
	/*	var _, err = ping(NetstackRequestGo{
			 WgIp:             "10.1.155.153",
			 PrivateKey:       "...",
			 PublicKey:        "...",
			 Endpoint:         "13.245.9.123:51822",
			 MetadataEndpoint: "http://10.1.0.1:51830",
			 Dns:              "1.1.1.1",
			 IpVersion:        4,
			 //PingHosts:          nil,
			 //PingIps:            nil,
			 //NumPing:            0,
			 //SendTimeoutSec:     0,
			 //RecvTimeoutSec:     0,
			 //DownloadTimeoutSec: 0,
			 MetadataTimeoutSec: 5,
			 //AwgArgs:            "",
		 })

		 if err != nil {
			 log.Fatal(err)
		 }
	*/
}
