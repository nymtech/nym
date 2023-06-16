package main

import (
	"log"
	"net/http"
)

func main() {
	fs := http.FileServer(http.Dir("./internal-static"))
	http.Handle("/", fs)

	log.Println("Listening on http://localhost:3000/index.html")
	err := http.ListenAndServe(":3000", nil)
	if err != nil {
		log.Fatal(err)
	}
}
