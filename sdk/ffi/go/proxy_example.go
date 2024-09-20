package main

import (
	"bufio"
	"fmt"
	"net"
	"nymffi/go-nym/bindings"
	"os"
	"time"
)

func runProxyClient() {
	run_err := bindings.RunProxyClient()
	if run_err != nil {
		fmt.Println(run_err)
		return
	}
}

func runProxyServer() {
	run_err := bindings.RunProxyServer()
	if run_err != nil {
		fmt.Println(run_err)
		return
	}
}

// connects to the proxy server and listens out for incoming as you would with a normal tcp connection
func startTcpListener() {
	ln, err := net.Listen("tcp", ":9000")
	if err != nil {
		fmt.Println(err)
		return
	}

	for {
		conn, err := ln.Accept()
		if err != nil {
			fmt.Println(err)
			continue
		}

		go handleConnection(conn)
	}
}

func handleConnection(conn net.Conn) {
	defer conn.Close()

	buf := make([]byte, 1024)
	_, err := conn.Read(buf)
	if err != nil {
		fmt.Println(err)
		return
	}

	fmt.Printf("Server-side tcp received: %s", buf)

	_, err = conn.Write(buf)
	if err != nil {
		fmt.Println(err)
		return
	}
}

func main() {

	// our mixnet client config file defining which network to use
	var env_path = "../../../envs/canary.env"
	// the tcp socket our server communicates with - the remote host your client is trying to hit
	var upstreamAddress = "127.0.0.1:9000"
	// where the keys and persistent storage for SURBs is located (this path will be prepended with the value of $HOME in the rust lib)
	var configDir = "/tmp/go-proxy-server-example"
	// tcp socket port our proxy client communicates with
	var clientPort = "8080"
	// timeout for ephemeral client to shutdown connection after sending Close message enum once it has sent all of the other messages (in seconds): this is used by the ProxyServer for session management
	var clientTimeout uint64 = 60

	bindings.InitLogging()

	// checking loading proper env
	file, err := os.Open(env_path)
	if err != nil {
		fmt.Println("Error opening file:", err)
		return
	}
	defer file.Close()
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		fmt.Println(scanner.Text())
	}

	// init a proxy server
	build_serv_err := bindings.NewProxyServer(upstreamAddress, configDir, &env_path)
	if build_serv_err != nil {
		fmt.Println(build_serv_err)
		return
	}

	// get proxy addr
	proxyAddr, get_addr_err := bindings.ProxyServerAddress()
	if get_addr_err != nil {
		fmt.Println("(Go) Error:", get_addr_err)
		return
	}
	fmt.Println("(Go) server address:")
	fmt.Println(proxyAddr)

	// run it in a goroutine
	go runProxyServer()

	// initialise a proxy client
	build_err := bindings.NewProxyClient(proxyAddr, "127.0.0.1", clientPort, clientTimeout, &env_path)
	if build_err != nil {
		fmt.Println(build_err)
		return
	}

	// run it in a goroutine
	go runProxyClient()

	// connect 'server-side' tcp socket to ProxyServer
	go startTcpListener()

	// send a oneshot message, wait for the echo, and close. you will see the session uuid info and the fact that the proxy_client logs it will be closing the session in <clientTimeout>.
	conn, err := net.Dial("tcp", "localhost:8080")
	if err != nil {
		fmt.Println(err)
		return
	}
	_, err = conn.Write([]byte("Hello, server: oneshot ping\n"))
	if err != nil {
		fmt.Println(err)
		return
	}

	buf := make([]byte, 1024)
	_, read_err := conn.Read(buf)
	if read_err != nil {
		fmt.Println(read_err)
		return
	}
	fmt.Printf("Client-side tcp received: %s", buf)
	conn.Close()

	// TODO replace with ctrl+c
	time.Sleep(60 * time.Second)
	fmt.Println("(Go) end go example")
}
