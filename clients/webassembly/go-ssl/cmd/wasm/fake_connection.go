// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"crypto/tls"
	"encoding/hex"
	"fmt"
	"io"
	"net"
	"strconv"
	"sync/atomic"
	"syscall/js"
	"time"
)

var currentConnection *ManagedConnection

func ensureManagedTls() bool {
	if currentConnection == nil {
		return false
	}
	return currentConnection.tlsConn != nil
}

func ensureManagedPlain() bool {
	if currentConnection == nil {
		return false
	}
	return currentConnection.plainConn != nil
}

type ManagedConnection struct {
	// ughhh... Go doesn't have proper enums...

	// TODO: this might be the wrong design. maybe there should only be something like *tlsHandshakeInProgressConn
	tlsConn   *tls.Conn
	plainConn net.Conn

	connectionInjector ConnectionInjector
}

func NewTlsManagedConnection(conn *tls.Conn, injector ConnectionInjector) ManagedConnection {
	return ManagedConnection{
		tlsConn:            conn,
		plainConn:          nil,
		connectionInjector: injector,
	}
}

func NewPlainManagedConnection(conn net.Conn, injector ConnectionInjector) ManagedConnection {
	return ManagedConnection{
		tlsConn:            nil,
		plainConn:          conn,
		connectionInjector: injector,
	}
}

// ConnectionInjector controls data that goes over corresponding FakeConnection
type ConnectionInjector struct {
	injectedServerData chan []byte
	closedRemote       *atomic.Bool
	//createdClientData  chan []byte
}

// FakeConnection is a type implementing net.Conn interface that allows us
// to inspect and control bytes that would normally go onto the wire
type FakeConnection struct {
	injectedServerData chan []byte
	//createdClientData  chan []byte
	closedRemote *atomic.Bool

	connectionId ConnectionId

	incompleteReads chan []byte
}

func NewFakeConnection(connectionId ConnectionId) (FakeConnection, ConnectionInjector) {
	injectedServerData := make(chan []byte, 10)
	//createdClientData := make(chan []byte, 10)

	closedRemote := &atomic.Bool{}

	conn := FakeConnection{
		injectedServerData: injectedServerData,
		connectionId:       connectionId,
		closedRemote:       closedRemote,
		//createdClientData:  createdClientData,
		incompleteReads: make(chan []byte, 1),
	}

	inj := ConnectionInjector{
		injectedServerData: injectedServerData,
		closedRemote:       closedRemote,
		//createdClientData:  createdClientData,
	}

	return conn, inj
}

func setupFakePlainConn(connectionId ConnectionId) ManagedConnection {
	conn, inj := NewFakeConnection(connectionId)
	return NewPlainManagedConnection(conn, inj)
}

func (conn *FakeConnection) readAndBuffer(in []byte, out []byte) (int, error) {
	buf := bytes.NewReader(in)
	n, err := buf.Read(out)

	remaining := buf.Len()
	if remaining > 0 {
		leftover := make([]byte, remaining)
		_, _ = buf.Read(leftover)
		conn.incompleteReads <- leftover
	}

	encoded := hex.EncodeToString(out[:n])
	Debug("READING INJECTED >>> %s\n", encoded)
	return n, err
}

func (conn FakeConnection) Read(p []byte) (int, error) {
	fmt.Printf("remote status: %v\n", conn.closedRemote.Load())
	select {
	// see if we have any leftover data from the previous read
	case incomplete := <-conn.incompleteReads:
		Debug("reading previously incomplete data")
		return conn.readAndBuffer(incomplete, p)
	default:
		select {
		// if we have any data: do read it
		case injectedData := <-conn.injectedServerData:
			return conn.readAndBuffer(injectedData, p)
		default:
			// otherwise see if the socket is closed
			if conn.closedRemote.Load() {
				println("remote is closed")
				return 0, io.EOF
			} else {
				Debug("waiting for data to read...")
				// wait for the data

				// TODO: what if we received information about closed socket here?
				data := <-conn.injectedServerData
				if len(data) == 0 {
					println("read zero - are we done?")
					return 0, io.EOF
				}
				return conn.readAndBuffer(data, p)
			}
		}
	}
}

func (conn FakeConnection) Write(p []byte) (int, error) {
	encoded := hex.EncodeToString(p)
	Debug("WRITING TO 'REMOTE' >>> %s\n", encoded)

	// "data" is a byte slice, so we need to convert it to a JS Uint8Array object
	arrayConstructor := js.Global().Get("Uint8Array")
	dataJS := arrayConstructor.New(len(p))
	js.CopyBytesToJS(dataJS, p)

	connId := strconv.FormatUint(conn.connectionId, 10)

	// this returns a promise...
	// TODO: DEAL with the promise...
	println("called rust code that returns a promise")
	js.Global().Call("send_client_data", connId, dataJS)
	println("but we're kinda ignoring it")

	//	js.Global().Call("onClientData", encoded)

	//conn.createdClientData <- p
	return len(p), nil
}
func (conn FakeConnection) Close() error {
	Warn("TODO: implement close FakeConnection")
	return nil
}
func (conn FakeConnection) LocalAddr() net.Addr {
	Warn("TODO: implement LocalAddr FakeConnection")
	return nil
}
func (conn FakeConnection) RemoteAddr() net.Addr {
	Warn("TODO: implement RemoteAddr FakeConnection")
	return nil
}
func (conn FakeConnection) SetDeadline(t time.Time) error {
	Info("Setting deadline to %v\n", t)

	Warn("TODO: implement SetDeadline FakeConnection")
	return nil
}
func (conn FakeConnection) SetReadDeadline(t time.Time) error {
	Info("Setting read deadline to %v\n", t)

	Warn("TODO: implement SetReadDeadline FakeConnection")
	return nil
}
func (conn FakeConnection) SetWriteDeadline(t time.Time) error {
	Info("Setting wrtite deadline to %v\n", t)

	Warn("TODO: implement SetWriteDeadline FakeConnection")
	return nil
}
