/* SPDX-License-Identifier: GPL-3.0-only
 *
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 *
 * UDP forwarder for two-hop WireGuard tunneling.
 * Copied from nym-vpn-client/wireguard/libwg/forwarders/udp.go
 */

package main

import (
	"log"
	"net"
	"net/netip"
	"sync"
	"time"

	"github.com/amnezia-vpn/amneziawg-go/device"
	"github.com/amnezia-vpn/amneziawg-go/tun/netstack"
	"gvisor.dev/gvisor/pkg/tcpip/adapters/gonet"
)

const UDP_WRITE_TIMEOUT = time.Duration(5) * time.Second
const MAX_UDP_DATAGRAM_LEN = 65535

type UDPForwarderConfig struct {
	// Listen port for incoming UDP traffic.
	// For IPv4 endpoint, the listening port is bound to 127.0.0.1, for IPv6 it's ::1.
	ListenPort uint16

	// Client port on loopback from which the incoming connection will be received.
	// Only packets from this port will be passed through to the endpoint.
	ClientPort uint16

	// Endpoint to connect to over netstack
	Endpoint netip.AddrPort
}

// UDP forwarder that creates a bidirectional in-tunnel connection between a local and remote UDP endpoints
type UDPForwarder struct {
	logger *device.Logger

	// Netstack tunnel
	tnet *netstack.Net

	// UDP listener that receives inbound traffic piped to the remote endpoint
	listener *net.UDPConn

	// Outbound connection to the remote endpoint over the entry tunnel
	outbound *gonet.UDPConn

	// Wait group used to signal when all goroutines have finished execution
	waitGroup *sync.WaitGroup

	// In netstack mode, conn.NewDefaultBind() doesn't honor listen_port IPC setting,
	// so we learn the actual client address from the first inbound packet.
	// This is protected by clientAddrMu and signaled via clientAddrCond.
	clientAddrMu   sync.Mutex
	clientAddrCond *sync.Cond
	learnedClient  *net.UDPAddr
}

func NewUDPForwarder(config UDPForwarderConfig, tnet *netstack.Net, logger *device.Logger) (*UDPForwarder, error) {
	var listenAddr *net.UDPAddr
	var clientAddr *net.UDPAddr

	// Use the same ip protocol family as endpoint
	if config.Endpoint.Addr().Is4() {
		loopback := netip.AddrFrom4([4]byte{127, 0, 0, 1})
		listenAddr = net.UDPAddrFromAddrPort(netip.AddrPortFrom(loopback, config.ListenPort))
		clientAddr = net.UDPAddrFromAddrPort(netip.AddrPortFrom(loopback, config.ClientPort))
	} else {
		listenAddr = net.UDPAddrFromAddrPort(netip.AddrPortFrom(netip.IPv6Loopback(), config.ListenPort))
		clientAddr = net.UDPAddrFromAddrPort(netip.AddrPortFrom(netip.IPv6Loopback(), config.ClientPort))
	}

	listener, err := net.ListenUDP("udp", listenAddr)
	if err != nil {
		return nil, err
	}

	outbound, err := tnet.DialUDPAddrPort(netip.AddrPort{}, config.Endpoint)
	if err != nil {
		listener.Close()
		return nil, err
	}

	waitGroup := &sync.WaitGroup{}
	wrapper := &UDPForwarder{
		logger:        logger,
		tnet:          tnet,
		listener:      listener,
		outbound:      outbound,
		waitGroup:     waitGroup,
		learnedClient: nil,
	}
	wrapper.clientAddrCond = sync.NewCond(&wrapper.clientAddrMu)

	waitGroup.Add(2)
	go wrapper.routineHandleInbound(listener, outbound, clientAddr)
	go wrapper.routineHandleOutbound(listener, outbound, clientAddr)

	return wrapper, nil
}

func (w *UDPForwarder) GetListenAddr() net.Addr {
	return w.listener.LocalAddr()
}

func (w *UDPForwarder) Close() {
	// Close all connections. This should release any blocking ReadFromUDP() calls
	w.listener.Close()
	w.outbound.Close()

	// Wait for all routines to complete
	w.waitGroup.Wait()
}

func (w *UDPForwarder) Wait() {
	w.waitGroup.Wait()
}

func (w *UDPForwarder) routineHandleInbound(inbound *net.UDPConn, outbound *gonet.UDPConn, clientAddr *net.UDPAddr) {
	defer w.waitGroup.Done()
	defer outbound.Close()

	inboundBuffer := make([]byte, MAX_UDP_DATAGRAM_LEN)

	w.logger.Verbosef("udpforwarder(inbound): listening on %s (proxy to %s)", inbound.LocalAddr().String(), outbound.RemoteAddr().String())
	defer w.logger.Verbosef("udpforwarder(inbound): closed")

	for {
		// Receive the WireGuard packet from local port
		bytesRead, senderAddr, err := inbound.ReadFromUDP(inboundBuffer)
		if err != nil {
			w.logger.Errorf("udpforwarder(inbound): %s", err.Error())
			return
		}

		log.Printf("udpforwarder(inbound): received %d bytes from %s", bytesRead, senderAddr.String())

		// Only accept packets from loopback
		if !senderAddr.IP.IsLoopback() {
			log.Printf("udpforwarder(inbound): drop packet from non-loopback: %s", senderAddr.String())
			continue
		}

		// Learn the client address from the first packet and notify the outbound handler
		// In netstack mode, conn.NewDefaultBind() doesn't honor listen_port IPC setting,
		// so we learn the actual client address from the first inbound packet.
		w.clientAddrMu.Lock()
		if w.learnedClient == nil {
			w.learnedClient = senderAddr
			log.Printf("udpforwarder(inbound): learned client addr: %s", w.learnedClient.String())
			w.clientAddrCond.Broadcast() // Signal outbound handler
		}
		learnedPort := w.learnedClient.Port
		w.clientAddrMu.Unlock()

		// Drop packet from unknown sender (different port than the learned client)
		if senderAddr.Port != learnedPort {
			log.Printf("udpforwarder(inbound): drop packet from unknown sender: %s, expected port: %d.", senderAddr.String(), learnedPort)
			continue
		}

		log.Printf("udpforwarder(inbound): forwarding %d bytes to exit gateway", bytesRead)

		// Set write timeout for outbound
		deadline := time.Now().Add(UDP_WRITE_TIMEOUT)
		err = outbound.SetWriteDeadline(deadline)
		if err != nil {
			w.logger.Errorf("udpforwarder(inbound): %s", err.Error())
			return
		}

		// Forward the packet over the outbound connection via another WireGuard tunnel
		bytesWritten, err := outbound.Write(inboundBuffer[:bytesRead])
		if err != nil {
			w.logger.Errorf("udpforwarder(inbound): %s", err.Error())
			return
		}

		if bytesWritten != bytesRead {
			w.logger.Errorf("udpforwarder(inbound): wrote %d bytes, expected %d", bytesWritten, bytesRead)
		}
	}
}

func (w *UDPForwarder) routineHandleOutbound(inbound *net.UDPConn, outbound *gonet.UDPConn, clientAddr *net.UDPAddr) {
	defer w.waitGroup.Done()
	defer inbound.Close()

	remoteAddr := outbound.RemoteAddr().(*net.UDPAddr)
	w.logger.Verbosef("udpforwarder(outbound): dial %s", remoteAddr.String())
	defer w.logger.Verbosef("udpforwarder(outbound): closed")

	outboundBuffer := make([]byte, MAX_UDP_DATAGRAM_LEN)

	for {
		// Receive WireGuard packet from remote server
		bytesRead, senderAddr, err := outbound.ReadFrom(outboundBuffer)
		if err != nil {
			w.logger.Errorf("udpforwarder(outbound): %s", err.Error())
			return
		}
		// Cast net.Addr to net.UDPAddr
		senderUDPAddr := senderAddr.(*net.UDPAddr)

		log.Printf("udpforwarder(outbound): received %d bytes from %s", bytesRead, senderUDPAddr.String())

		// Drop packet from unknown sender.
		if !senderUDPAddr.IP.Equal(remoteAddr.IP) || senderUDPAddr.Port != remoteAddr.Port {
			log.Printf("udpforwarder(outbound): drop packet from unknown sender: %s, expected: %s", senderUDPAddr.String(), remoteAddr.String())
			continue
		}

		// Wait for the learned client address from the inbound handler
		// This ensures we send responses to the actual client port (which may differ from expected)
		w.clientAddrMu.Lock()
		for w.learnedClient == nil {
			w.clientAddrCond.Wait()
		}
		targetClient := w.learnedClient
		w.clientAddrMu.Unlock()

		log.Printf("udpforwarder(outbound): forwarding %d bytes to client %s", bytesRead, targetClient.String())

		// Set write timeout for inbound
		deadline := time.Now().Add(UDP_WRITE_TIMEOUT)
		err = inbound.SetWriteDeadline(deadline)
		if err != nil {
			w.logger.Errorf("udpforwarder(outbound): %s", err.Error())
			return
		}

		// Forward packet from remote to local client (using learned address)
		bytesWritten, err := inbound.WriteToUDP(outboundBuffer[:bytesRead], targetClient)
		if err != nil {
			w.logger.Errorf("udpforwarder(outbound): %s", err.Error())
			return
		}

		if bytesWritten != bytesRead {
			w.logger.Errorf("udpforwarder(outbound): wrote %d bytes, expected %d", bytesWritten, bytesRead)
		}
	}
}
