// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package state

import (
	"bytes"
	"crypto/tls"
	"go-mix-conn/internal/bridge/rust_bridge"
	"go-mix-conn/internal/log"
	"go-mix-conn/internal/sslhelpers"
	"go-mix-conn/internal/types"
	"io"
	"net"
	"os"
	"sync"
	"time"
)

// InjectedData is, well, the injected server data that came from the mixnet into this connection
type InjectedData struct {
	ServerData <-chan []byte
	//RemoteClosed <-chan bool
	RemoteDone <-chan struct{}

	RemoteError <-chan error
}

// ConnectionInjector controls data that goes over corresponding FakeConnection
type ConnectionInjector struct {
	ServerData chan<- []byte
	RemoteDone chan<- struct{}
	//RemoteClosed chan<- bool
	RemoteError chan<- error
}

// FakeConnection is a type implementing net.Conn interface that allows us
// to inspect and control bytes that would normally go onto the wire
type FakeConnection struct {
	requestId     types.RequestId
	remoteAddress string
	data          *InjectedData

	localDone chan struct{}

	readDeadline  connDeadline
	writeDeadline connDeadline

	pendingReads chan []byte
}

// connDeadline is an abstraction for handling timeouts.
// source: https://github.com/golang/go/blob/release-branch.go1.20/src/net/pipe.go#L15
type connDeadline struct {
	mu     sync.Mutex // Guards timer and cancel
	timer  *time.Timer
	cancel chan struct{} // Must be non-nil
}

func makeConnDeadline() connDeadline {
	return connDeadline{cancel: make(chan struct{})}
}

// set sets the point in time when the deadline will time out.
// A timeout event is signaled by closing the channel returned by waiter.
// Once a timeout has occurred, the deadline can be refreshed by specifying a
// t value in the future.
//
// A zero value for t prevents timeout.
// source: https://github.com/golang/go/blob/release-branch.go1.20/src/net/pipe.go#L31
func (d *connDeadline) set(t time.Time) {
	d.mu.Lock()
	defer d.mu.Unlock()

	if d.timer != nil && !d.timer.Stop() {
		<-d.cancel // Wait for the timer callback to finish and close cancel
	}
	d.timer = nil

	// Time is zero, then there is no deadline.
	closed := isClosedChan(d.cancel)
	if t.IsZero() {
		if closed {
			d.cancel = make(chan struct{})
		}
		return
	}

	// Time in the future, setup a timer to cancel in the future.
	if dur := time.Until(t); dur > 0 {
		if closed {
			d.cancel = make(chan struct{})
		}
		d.timer = time.AfterFunc(dur, func() {
			close(d.cancel)
		})
		return
	}

	// Time in the past, so close immediately.
	if !closed {
		close(d.cancel)
	}
}

func (d *connDeadline) wait() chan struct{} {
	d.mu.Lock()
	defer d.mu.Unlock()
	return d.cancel
}

func isClosedChan(c <-chan struct{}) bool {
	select {
	case <-c:
		return true
	default:
		return false
	}
}

// NewFakeConnection creates a new FakeConnection that implements net.Conn interface alongside
// handlers for injecting data into it
func NewFakeConnection(requestId types.RequestId, remoteAddress string) (*FakeConnection, ConnectionInjector) {
	serverData := make(chan []byte, 10)
	//remoteClosed := make(chan bool, 1)
	remoteError := make(chan error, 1)

	localDone := make(chan struct{})
	remoteDone := make(chan struct{})

	inj := ConnectionInjector{
		ServerData: serverData,
		//RemoteClosed: remoteClosed,
		RemoteDone:  remoteDone,
		RemoteError: remoteError,
	}

	conn := &FakeConnection{
		data: &InjectedData{
			ServerData: serverData,
			//RemoteClosed: remoteClosed,
			RemoteDone:  remoteDone,
			RemoteError: remoteError,
		},
		requestId:     requestId,
		remoteAddress: remoteAddress,
		pendingReads:  make(chan []byte, 1),
		localDone:     localDone,
		readDeadline:  makeConnDeadline(),
		writeDeadline: makeConnDeadline(),
	}

	return conn, inj
}

// NewFakeTlsConn wraps a FakeConnection with all the TLS magic
// note: this returns a tls.Conn in the pre-handshake state
func NewFakeTlsConn(connectionId types.RequestId, remoteAddress string) (*tls.Conn, ConnectionInjector) {
	host, _, err := net.SplitHostPort(remoteAddress)
	if err != nil {
		panic("todo")
	}
	conn, inj := NewFakeConnection(connectionId, remoteAddress)
	tlsConfig := sslhelpers.TlsConfig(host)
	tlsConn := tls.Client(conn, &tlsConfig)
	return tlsConn, inj
}

func (conn *FakeConnection) readAndBuffer(in []byte, out []byte) (int, error) {
	buf := bytes.NewReader(in)
	n, err := buf.Read(out)

	remaining := buf.Len()
	if remaining > 0 {
		leftover := make([]byte, remaining)
		_, _ = buf.Read(leftover)
		conn.pendingReads <- leftover
	}

	log.Debug("READING INJECTED %d bytes <<<", n)
	return n, err
}

// TODO: so many EOF edge cases here...
func (conn *FakeConnection) Read(p []byte) (int, error) {
	switch {
	case isClosedChan(conn.localDone):
		return 0, io.ErrClosedPipe
	//case isClosedChan(conn.data.RemoteDone):
	//	return 0, io.EOF
	case isClosedChan(conn.readDeadline.wait()):
		return 0, os.ErrDeadlineExceeded
	}

	// TODO: is there really no better way for priority chan reads?
	select {
	// see if we have any leftover data from the previous read
	case incomplete := <-conn.pendingReads:
		log.Trace("reading previously incomplete data")
		return conn.readAndBuffer(incomplete, p)
	default:
		// reason for this extra select:
		// if we have BOTH server data and closing information - you HAVE TO use up the data first
		select {
		// if we have any data: do read it
		case injectedData := <-conn.data.ServerData:
			log.Trace("server data")
			return conn.readAndBuffer(injectedData, p)
		default:
			// we wait for either some data, closing info, an error or timeout
			select {
			case data := <-conn.data.ServerData:
				if len(data) == 0 {
					return 0, io.EOF
				}
				return conn.readAndBuffer(data, p)
			case err := <-conn.data.RemoteError:
				return 0, err
			case <-conn.localDone:
				return 0, io.ErrClosedPipe
			case <-conn.data.RemoteDone:
				return 0, io.EOF
			case <-conn.readDeadline.wait():
				return 0, os.ErrDeadlineExceeded
			}
		}
	}
}

func (conn *FakeConnection) Write(p []byte) (int, error) {
	log.Debug("WRITING %d bytes TO 'REMOTE' >>> \n", len(p))

	switch {
	case isClosedChan(conn.localDone):
		return 0, io.ErrClosedPipe
	//case isClosedChan(conn.data.RemoteDone):
	//	return 0, io.EOF
	case isClosedChan(conn.readDeadline.wait()):
		return 0, os.ErrDeadlineExceeded
	}

	// TODO: I guess the deadline should take into consideration the recipient actually getting the packet...
	err := rust_bridge.RsSendClientData(conn.requestId, p)
	if err != nil {
		return 0, err
	} else {
		return len(p), nil
	}
}

func (conn *FakeConnection) Close() error {
	log.Debug("closing FakeConnection")

	close(conn.localDone)
	ActiveRequests.Remove(conn.requestId)

	// TODO: if we already received information about remote being closed,
	// do we have to send a socks5 closing packet?
	return rust_bridge.RsFinishMixnetConnection(conn.requestId)
}

func (conn *FakeConnection) LocalAddr() net.Addr {
	log.Warn("TODO: implement LocalAddr FakeConnection")
	return nil
}

func (conn *FakeConnection) RemoteAddr() net.Addr {
	log.Warn("TODO: implement RemoteAddr FakeConnection")
	return nil
}

func (conn *FakeConnection) SetDeadline(t time.Time) error {
	log.Trace("Setting deadline to %v\n", t)

	if isClosedChan(conn.localDone) || isClosedChan(conn.data.RemoteDone) {
		return io.ErrClosedPipe
	}

	conn.readDeadline.set(t)
	conn.writeDeadline.set(t)

	return nil
}

func (conn *FakeConnection) SetReadDeadline(t time.Time) error {
	log.Trace("Setting read deadline to %v\n", t)

	if isClosedChan(conn.localDone) || isClosedChan(conn.data.RemoteDone) {
		return io.ErrClosedPipe
	}

	conn.readDeadline.set(t)

	return nil
}

func (conn *FakeConnection) SetWriteDeadline(t time.Time) error {
	log.Trace("Setting write deadline to %v\n", t)

	if isClosedChan(conn.localDone) || isClosedChan(conn.data.RemoteDone) {
		return io.ErrClosedPipe
	}

	conn.writeDeadline.set(t)

	return nil
}
