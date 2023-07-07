// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package helpers

import (
	"errors"
	"fmt"
	"go-mix-conn/internal/external"
	"golang.org/x/net/http/httpguts"
	"sort"
	"strconv"
	"strings"
	"syscall/js"
)

func ParseRequestId(raw js.Value) (uint64, error) {
	if raw.Type() != js.TypeString {
		return 0, errors.New("the received raw request id was not a string")
	}

	return strconv.ParseUint(raw.String(), 10, 64)
}

func IntoGoBytes(raw js.Value) ([]byte, error) {
	if raw.Type() != js.TypeObject {
		return nil, errors.New("the received 'bytes' are not an object")
	}
	lenProp := raw.Get("length")
	if lenProp.Type() != js.TypeNumber {
		return nil, errors.New("the received 'bytes' object does not have a numerical 'length' property")
	}
	n := lenProp.Int()
	bytes := make([]byte, n)

	// TODO: somehow check that the object is an Uint8Array or Uint8ClampedArray
	copied := js.CopyBytesToGo(bytes, raw)
	if copied != n {
		// I don't see how this could ever be reached, thus panic
		panic("somehow copied fewer bytes from JavaScript into Go than what we specified as our buffer")
	}

	return bytes, nil
}

func IntoJsBytes(raw []byte) js.Value {
	// "data" is a byte slice, so we need to convert it to a JS Uint8Array object
	arrayConstructor := js.Global().Get("Uint8Array")
	jsBytes := arrayConstructor.New(len(raw))
	js.CopyBytesToJS(jsBytes, raw)
	return jsBytes
}

func GetStringProperty(obj *js.Value, name string) (string, error) {
	val := obj.Get(name)
	if val.Type() != js.TypeString {
		return "", errors.New(fmt.Sprintf("the property %s is not a string", name))
	}
	return val.String(), nil
}

func IsToken(raw string) bool {
	if len(raw) == 0 {
		return false
	}
	for _, b := range []byte(raw) {
		if !httpguts.IsTokenRune(rune(b)) {
			return false
		}
	}
	return true
}

func ByteLowercase(s string) string {
	return strings.Map(byteLowercaseOne, s)
}

func byteLowercaseOne(asciiRune rune) rune {
	const toLower = 'a' - 'A'
	if 'A' <= asciiRune && asciiRune <= 'Z' {
		return asciiRune + toLower
	}
	return asciiRune
}

func SortedByteLowercase(s []string) []string {
	lowercase := make([]string, len(s))
	for i := 0; i < len(s); i++ {
		lowercase[i] = ByteLowercase(s[i])
	}
	sort.Strings(lowercase)
	return lowercase
}

func Contains(s []string, str string) bool {
	for _, v := range s {
		if v == str {
			return true
		}
	}

	return false
}

func Unique(s []string) []string {
	uniqueSet := external.NewSet(s...)

	uniqueSlice := make([]string, len(uniqueSet))
	for v := range uniqueSet {
		uniqueSlice = append(uniqueSlice, v)
	}
	return uniqueSlice
}

func IntoAnySlice(v []js.Value) []any {
	s := make([]any, len(v))
	for i, x := range v {
		s[i] = x
	}
	return s
}
