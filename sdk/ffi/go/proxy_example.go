package main

import (
	"fmt"
	"nymffi/go-nym/bindings"
	"time"
)

func runProxyClient() {
	run_err := bindings.RunProxyClient()
	if run_err != nil {
		fmt.Println(run_err)
		return
	}
}

func main() {

	var env_path = "../../../envs/canary.env"

	bindings.InitLogging()

	// checking loading proper env
	// file, err := os.Open(env_path)
	// if err != nil {
	// 	fmt.Println("Error opening file:", err)
	// 	return
	// }
	// defer file.Close()
	// scanner := bufio.NewScanner(file)
	// for scanner.Scan() {
	// 	fmt.Println(scanner.Text())
	// }

	// TODO
	// init a proxy server
	// get address
	// give addr to proxy client
	// create goroutine & start it
	// pipe messages & read async

	// initialise a proxy client
	build_err := bindings.NewProxyClient("12Uz16Kp2s4aX5Booi1j1g9Fuau6Lqc82xR6j2n6i6Gy.DwudGgjfSbPJ6SBJpArEgS3KJXt56hR8cnrCAUB1GNUX@63ctaex57EvjJZu92jT2ve2ULgmjVYAQph83qNjMpFDZ", "127.0.0.1", "8080", 60, &env_path)
	if build_err != nil {
		fmt.Println(build_err)
		return
	}

	// run it in a thread
	go runProxyClient()

	// sleep so that the nym client processes can catch up - in reality you'd have another process
	// running to keep logging going, so this is only necessary for this reference
	// TODO replace with ctrl+c
	time.Sleep(60 * time.Second)
	fmt.Println("(Go) end go example")
}
