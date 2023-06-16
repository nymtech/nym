//

package main

import (
	"bytes"
	_ "crypto/sha512"
	gotls "crypto/tls"
	"crypto/x509"
	"encoding/hex"
	"fmt"
	tls "github.com/refraction-networking/utls"
	"io"
	"net"
	"syscall/js"
	"time"
	_ "unsafe"
)

var done chan struct{}

func init() {
	done = make(chan struct{})
}

func main() {
	println("go wasm main start")
	//js.Global().Set("wasmFoomp", js.FuncOf(foomp))
	js.Global().Set("wasmClientHello", js.FuncOf(makeClientHello))
	js.Global().Set("wasmClientHelloUtls", js.FuncOf(makeClientHelloUtls))
	js.Global().Set("wasmInsertServerHello", js.FuncOf(insertServerHello))
	js.Global().Set("wasmPrintConnection", js.FuncOf(printConnection))
	js.Global().Set("goFoomp", js.FuncOf(printFoomp))
	<-done
	println("go wasm is done.")
}

func printFoomp(this js.Value, args []js.Value) interface{} {
	println("go foomp")
	return "foomp indeed"
}

/*
func HttpGetCustom(hostname string, addr string) (*http.Response, error) {
	config := tls.Config{ServerName: hostname}
	dialConn, err := net.DialTimeout("tcp", addr, dialTimeout)
	if err != nil {
		return nil, fmt.Errorf("net.DialTimeout error: %+v", err)
	}
	uTlsConn := tls.UClient(dialConn, &config, tls.HelloCustom)
	defer uTlsConn.Close()

	// do not use this particular spec in production
	// make sure to generate a separate copy of ClientHelloSpec for every connection
	spec := tls.ClientHelloSpec{
		TLSVersMax: tls.VersionTLS13,
		TLSVersMin: tls.VersionTLS10,
		CipherSuites: []uint16{
			tls.GREASE_PLACEHOLDER,
			tls.TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305,
			tls.TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
			tls.TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA,
			tls.TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA,
			tls.TLS_AES_128_GCM_SHA256, // tls 1.3
			tls.FAKE_TLS_DHE_RSA_WITH_AES_256_CBC_SHA,
			tls.TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
			tls.TLS_RSA_WITH_AES_256_CBC_SHA,
		},
		Extensions: []tls.TLSExtension{
			&tls.SNIExtension{},
			&tls.SupportedCurvesExtension{Curves: []tls.CurveID{tls.X25519, tls.CurveP256}},
			&tls.SupportedPointsExtension{SupportedPoints: []byte{0}}, // uncompressed
			&tls.SessionTicketExtension{},
			&tls.ALPNExtension{AlpnProtocols: []string{"myFancyProtocol", "http/1.1"}},
			&tls.SignatureAlgorithmsExtension{SupportedSignatureAlgorithms: []tls.SignatureScheme{
				tls.ECDSAWithP256AndSHA256,
				tls.ECDSAWithP384AndSHA384,
				tls.ECDSAWithP521AndSHA512,
				tls.PSSWithSHA256,
				tls.PSSWithSHA384,
				tls.PSSWithSHA512,
				tls.PKCS1WithSHA256,
				tls.PKCS1WithSHA384,
				tls.PKCS1WithSHA512,
				tls.ECDSAWithSHA1,
				tls.PKCS1WithSHA1}},
			&tls.KeyShareExtension{[]tls.KeyShare{
				{Group: tls.CurveID(tls.GREASE_PLACEHOLDER), Data: []byte{0}},
				{Group: tls.X25519},
			}},
			&tls.PSKKeyExchangeModesExtension{[]uint8{1}}, // pskModeDHE
			&tls.SupportedVersionsExtension{[]uint16{
				tls.VersionTLS13,
				tls.VersionTLS12,
				tls.VersionTLS11,
				tls.VersionTLS10}},
		},
		GetSessionID: nil,
	}
	err = uTlsConn.ApplyPreset(&spec)

	if err != nil {
		return nil, fmt.Errorf("uTlsConn.Handshake() error: %+v", err)
	}

	err = uTlsConn.Handshake()
	if err != nil {
		return nil, fmt.Errorf("uTlsConn.Handshake() error: %+v", err)
	}

	return httpGetOverConn(uTlsConn, uTlsConn.HandshakeState.ServerHello.AlpnProtocol)
}
*/

//// clientHandshakeWithOneState checks that exactly one expected state is set (1.2 or 1.3)
//// and performs client TLS handshake with that state
//func (c *UConn) clientHandshake(ctx context.Context) (err error) {
//	// [uTLS section begins]
//	hello := c.HandshakeState.Hello.getPrivatePtr()
//	defer func() { c.HandshakeState.Hello = hello.getPublicPtr() }()
//
//	sessionIsAlreadySet := c.HandshakeState.Session != nil
//
//	// after this point exactly 1 out of 2 HandshakeState pointers is non-nil,
//	// useTLS13 variable tells which pointer
//	// [uTLS section ends]
//
//	if c.config == nil {
//		c.config = defaultConfig()
//	}
//
//	// This may be a renegotiation handshake, in which case some fields
//	// need to be reset.
//	c.didResume = false
//
//	// [uTLS section begins]
//	// don't make new ClientHello, use hs.hello
//	// preserve the checks from beginning and end of makeClientHello()
//	if len(c.config.ServerName) == 0 && !c.config.InsecureSkipVerify && len(c.config.InsecureServerNameToVerify) == 0 {
//		return errors.New("tls: at least one of ServerName, InsecureSkipVerify or InsecureServerNameToVerify must be specified in the tls.Config")
//	}
//
//	nextProtosLength := 0
//	for _, proto := range c.config.NextProtos {
//		if l := len(proto); l == 0 || l > 255 {
//			return errors.New("tls: invalid NextProtos value")
//		} else {
//			nextProtosLength += 1 + l
//		}
//	}
//
//	if nextProtosLength > 0xffff {
//		return errors.New("tls: NextProtos values too large")
//	}
//
//	if c.handshakes > 0 {
//		hello.secureRenegotiation = c.clientFinished[:]
//	}
//	// [uTLS section ends]
//
//	cacheKey, session, earlySecret, binderKey, err := c.loadSession(hello)
//	if err != nil {
//		return err
//	}
//	if cacheKey != "" && session != nil {
//		defer func() {
//			// If we got a handshake failure when resuming a session, throw away
//			// the session ticket. See RFC 5077, Section 3.2.
//			//
//			// RFC 8446 makes no mention of dropping tickets on failure, but it
//			// does require servers to abort on invalid binders, so we need to
//			// delete tickets to recover from a corrupted PSK.
//			if err != nil {
//				c.config.ClientSessionCache.Put(cacheKey, nil)
//			}
//		}()
//	}
//
//	if !sessionIsAlreadySet { // uTLS: do not overwrite already set session
//		err = c.SetSessionState(session)
//		if err != nil {
//			return
//		}
//	}
//
//	if _, err := c.writeHandshakeRecord(hello, nil); err != nil {
//		return err
//	}
//
//	msg, err := c.readHandshake(nil)
//	if err != nil {
//		return err
//	}
//
//	serverHello, ok := msg.(*serverHelloMsg)
//	if !ok {
//		c.sendAlert(alertUnexpectedMessage)
//		return unexpectedMessageError(serverHello, msg)
//	}
//
//	if err := c.pickTLSVersion(serverHello); err != nil {
//		return err
//	}
//
//	// uTLS: do not create new handshakeState, use existing one
//	if c.vers == VersionTLS13 {
//		hs13 := c.HandshakeState.toPrivate13()
//		hs13.serverHello = serverHello
//		hs13.hello = hello
//		if !sessionIsAlreadySet {
//			hs13.earlySecret = earlySecret
//			hs13.binderKey = binderKey
//		}
//		hs13.ctx = ctx
//		// In TLS 1.3, session tickets are delivered after the handshake.
//		err = hs13.handshake()
//		if handshakeState := hs13.toPublic13(); handshakeState != nil {
//			c.HandshakeState = *handshakeState
//		}
//		return err
//	}
//
//	hs12 := c.HandshakeState.toPrivate12()
//	hs12.serverHello = serverHello
//	hs12.hello = hello
//	hs12.ctx = ctx
//	err = hs12.handshake()
//	if handshakeState := hs12.toPublic12(); handshakeState != nil {
//		c.HandshakeState = *handshakeState
//	}
//	if err != nil {
//		return err
//	}
//
//	// If we had a successful handshake and hs.session is different from
//	// the one already cached - cache a new one.
//	if cacheKey != "" && hs12.session != nil && session != hs12.session {
//		c.config.ClientSessionCache.Put(cacheKey, hs12.session)
//	}
//	return nil
//}

//// mmmm. go
////
////go:linkname getPublicPtr tls.getPublicPtr
//func (chm *tls.clientHelloMsg) getPublicPtr() *tls.PubClientHelloMsg

type clientHelloGenerator struct {
	value io.Writer
}

func (conn clientHelloGenerator) Read(p []byte) (int, error) {
	return 0, io.ErrClosedPipe
}
func (conn clientHelloGenerator) Write(p []byte) (int, error) {
	return conn.value.Write(p)
}
func (conn clientHelloGenerator) Close() error                       { return nil }
func (conn clientHelloGenerator) LocalAddr() net.Addr                { return nil }
func (conn clientHelloGenerator) RemoteAddr() net.Addr               { return nil }
func (conn clientHelloGenerator) SetDeadline(t time.Time) error      { return nil }
func (conn clientHelloGenerator) SetReadDeadline(t time.Time) error  { return nil }
func (conn clientHelloGenerator) SetWriteDeadline(t time.Time) error { return nil }

type fakeConn struct {
}

func (conn fakeConn) Read(p []byte) (int, error) {
	if readServerHello == true {
		return 0, io.ErrClosedPipe
	}

	readServerHello = true
	println("trying to read from connection")
	return fakeServerData.Read(p)
}
func (conn fakeConn) Write(p []byte) (int, error) {
	encoded := hex.EncodeToString(p)

	if bytes.Equal(p, oldHelloData) {
		println("writing ClientHello")
	} else {
		println("writing NOT clientHello")
		println(encoded)
	}

	return len(p), nil
}
func (conn fakeConn) Close() error                       { return nil }
func (conn fakeConn) LocalAddr() net.Addr                { return nil }
func (conn fakeConn) RemoteAddr() net.Addr               { return nil }
func (conn fakeConn) SetDeadline(t time.Time) error      { return nil }
func (conn fakeConn) SetReadDeadline(t time.Time) error  { return nil }
func (conn fakeConn) SetWriteDeadline(t time.Time) error { return nil }

// hehe this is so disgusting
// oh and I guess this code is also not the best
func makeClientHello(this js.Value, args []js.Value) interface{} {
	valueBuf := new(bytes.Buffer)
	conn := clientHelloGenerator{value: valueBuf}
	//uTlsConn := tls.UClient(conn, nil, )
	tlsConn := gotls.Client(conn, &gotls.Config{ServerName: "localhost"})
	err := tlsConn.Handshake()
	// err will obviously be non-nil since we failed to read anything
	fmt.Printf("expected err: %v\n", err)

	clientHello := valueBuf
	encoded := hex.EncodeToString(clientHello.Bytes())
	fmt.Printf("hello: %v", clientHello)

	return encoded
}

var oldHelloData []byte
var tlsConn *tls.UConn = nil
var fakeServerData = new(bytes.Buffer)
var readServerHello = false

func printConnection(this js.Value, args []js.Value) interface{} {
	if tlsConn == nil {
		println("no connection")
		return nil
	}

	println("ClientHelloBuilt: ", tlsConn.ClientHelloBuilt)
	println("State")
	fmt.Printf("ServerHello: %+v\n", tlsConn.HandshakeState.ServerHello)
	fmt.Printf("ClientHello: %+v\n", tlsConn.HandshakeState.Hello)
	fmt.Printf("MasterSecret: %+v\n", tlsConn.HandshakeState.MasterSecret)
	fmt.Printf("Session: %+v\n", tlsConn.HandshakeState.Session)
	fmt.Printf("State12: %+v\n", tlsConn.HandshakeState.State12)
	fmt.Printf("State13: %+v\n", tlsConn.HandshakeState.State13)
	fmt.Printf("conn: %+v\n", tlsConn.Conn)

	return nil
}

// https://www.ibm.com/docs/en/ztpf/1.1.0.15?topic=sessions-ssl-record-format
// https://www.analysisman.com/2021/06/wireshark-tls1.2.html
func recordHeader(tlsVersion uint16, payloadLen int) []byte {
	if payloadLen > 16384 {
		// let's not deal with chunking here
		panic("unsupported SSL record payload length")
	}

	if tlsVersion == tls.VersionTLS13 {
		panic("TODO: handle tls1.3 edge case")
	}

	// byte 0: SSL record type; in our case 0x16 - recordTypeHandshake
	// byte 1-2: TLS version. deprecated and ignored (https://datatracker.ietf.org/doc/html/rfc8446#page-79)
	// must be set to 0x0301 for compatibility purposes, unless using TLS 1.3
	// byte 3-4: length of data in the record
	return []byte{0x16, 0x03, 0x01, byte(payloadLen >> 8), byte(payloadLen)}
}

func makeClientHelloUtls(this js.Value, args []js.Value) interface{} {
	if tlsConn != nil {
		println("we have already generated ClientHello")
		return hex.EncodeToString(oldHelloData)
	}

	//uTlsConn := tls.UClient(conn, nil, )
	tlsConn = tls.UClient(fakeConn{}, &tls.Config{
		VerifyPeerCertificate: func(rawCerts [][]byte, verifiedChains [][]*x509.Certificate) error {
			println("verifying certs")
			return nil
		},
		// Set InsecureSkipVerify to skip the default validation we are
		// replacing. This will not disable VerifyConnection.
		InsecureSkipVerify: true,
		VerifyConnection: func(cs tls.ConnectionState) error {
			println("verifying conn")
			fmt.Printf("%+v\n", cs)
			return nil
		},
	}, tls.HelloRandomized)
	tlsConn.SetSNI("localhost")

	// this creates and marshals ClientHello
	err := tlsConn.BuildHandshakeState()
	if err != nil {
		panic(err)
	}

	//tlsConn.ApplyPreset()

	rawHello := tlsConn.HandshakeState.Hello.Raw
	tlsVersion := tlsConn.HandshakeState.Hello.Vers

	recordHeader := recordHeader(tlsVersion, len(rawHello))
	wireData := append(recordHeader, rawHello...)

	oldHelloData = wireData
	encoded := hex.EncodeToString(oldHelloData)

	println(encoded)
	println()

	return encoded
}

func insertServerHello(this js.Value, args []js.Value) interface{} {
	value := args[0].String()
	println("received", value)

	if tlsConn == nil || !tlsConn.ClientHelloBuilt {
		panic("did not call ClientHello")
	}

	decoded, err := hex.DecodeString(value)
	if err != nil {
		panic(err)
	}

	_, err = fakeServerData.Write(decoded)
	if err != nil {
		panic(err)
	}

	err = tlsConn.Handshake()
	if err != nil {
		panic(err)
	}

	println("handshake done?")
	return nil
}

func clientKeyExchange(this js.Value, args []js.Value) interface{} {
	return nil
}

//func foomp(this js.Value, args []js.Value) interface{} {
//	println("CCC")
//
//	//hello := &tls.PubClientHelloMsg{}
//	//helloBytes, err := hello.Marshal()
//	//if err != nil {
//	//	panic(err)
//	//}
//
//	//println(helloBytes)
//
//	write := new(bytes.Buffer)
//	read := new(bytes.Buffer)
//
//	conn := fakeConn{
//		reader: read,
//		writer: write,
//	}
//	uTlsConn := tls.UClient(conn, &tls.Config{
//		GetConfigForClient: func(info *tls.ClientHelloInfo) (*tls.Config, error) {
//			println("GetConfigForClient")
//			return nil, nil
//		},
//		ServerName: "localhost",
//		VerifyPeerCertificate: func(rawCerts [][]byte, verifiedChains [][]*x509.Certificate) error {
//			println("verifying certs")
//			return nil
//		},
//		// Set InsecureSkipVerify to skip the default validation we are
//		// replacing. This will not disable VerifyConnection.
//		InsecureSkipVerify: true,
//		VerifyConnection: func(cs tls.ConnectionState) error {
//			println("verifying conn")
//			fmt.Printf("%v\n", cs)
//			return nil
//		},
//	}, tls.HelloRandomized)
//
//	err := uTlsConn.Handshake()
//	if err != nil {
//		panic(err)
//	}
//
//	//buf := new(bytes.Buffer)
//	//conn := fakeConn{reader: buf}
//	//err := tls.Client(conn, &tls.Config{
//	//	GetConfigForClient:
//	//})
//
//	//tls.Client(fakeConn{reade})
//
//	//err := tls.Server(fakeConn{reader: reader}, &tls.Config{
//	//	GetConfigForClient: func(argHello *tls.ClientHelloInfo) (*tls.Config, error) {
//	//		hello = new(tls.ClientHelloInfo)
//	//		*hello = *argHello
//	//		return nil, nil
//	//	},
//	//}).Handshake()
//	return "ret"
//	//
//	//return helloBytes
//	//
//	////dialConn, err := net.DialTimeout("tcp", addr, dialTimeout)
//	////if err != nil {
//	////	return nil, fmt.Errorf("net.DialTimeout error: %+v", err)
//	////}
//	////uTlsConn := tls.UClient(dialConn, nil, tls.HelloGolang)
//	////
//	////handshakeState := tls.PubClientHandshakeState{C: nil, Hello: &tls.PubClientHelloMsg{}}
//	//
//	////_ = &tls.Config{
//	////	VerifyPeerCertificate: func(rawCerts [][]byte, verifiedChains [][]*x509.Certificate) error {
//	////		println("verifying certs")
//	////		return nil
//	////	},
//	////	// Set InsecureSkipVerify to skip the default validation we are
//	////	// replacing. This will not disable VerifyConnection.
//	////	InsecureSkipVerify: true,
//	////	VerifyConnection: func(cs tls.ConnectionState) error {
//	////		println("verifying conn")
//	////		return nil
//	////	},
//	////}
//	////
//	////tls.DialWithDialer()
//	//
//	//return "test"
//}

//type CustomDialer struct {
//	inner net.Dialer
//}

//type fakeConn struct {
//	reader io.Reader
//	writer io.Writer
//}
//
//func (conn fakeConn) Read(p []byte) (int, error) {
//	println("READING")
//	return conn.reader.Read(p)
//}
//func (conn fakeConn) Write(p []byte) (int, error) {
//
//	fmt.Printf("WRITING %v\n", p)
//	return conn.writer.Write(p)
//}
//func (conn fakeConn) Close() error                       { return nil }
//func (conn fakeConn) LocalAddr() net.Addr                { return nil }
//func (conn fakeConn) RemoteAddr() net.Addr               { return nil }
//func (conn fakeConn) SetDeadline(t time.Time) error      { return nil }
//func (conn fakeConn) SetReadDeadline(t time.Time) error  { return nil }
//func (conn fakeConn) SetWriteDeadline(t time.Time) error { return nil }
