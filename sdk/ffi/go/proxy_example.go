package main

import (
	"fmt"
	"nymffi/go-nym/bindings"
	"time"
)

func main() {

	var env_path = "../../../envs/canary.env"

	bindings.InitLogging()

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

	// initialise a proxy client
	err := bindings.NewProxyClient("12Uz16Kp2s4aX5Booi1j1g9Fuau6Lqc82xR6j2n6i6Gy.DwudGgjfSbPJ6SBJpArEgS3KJXt56hR8cnrCAUB1GNUX@63ctaex57EvjJZu92jT2ve2ULgmjVYAQph83qNjMpFDZ", "127.0.0.1", "8080", 60, &env_path)
	if err != nil {
		fmt.Println(err)
		return
	}

	// // sleep so that the nym client processes can catch up - in reality you'd have another process
	// // running to keep logging going, so this is only necessary for this reference
	time.Sleep(60 * time.Second)
	fmt.Println("(Go) end go example")
}
