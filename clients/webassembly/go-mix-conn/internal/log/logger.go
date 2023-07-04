// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package log

import (
	"fmt"
	"runtime"
	"syscall/js"
)

// thanks to the amazing capabilities of this awesome language,
// we couldn't put it in the `jstypes` alongside other globals
// as it created an unresolvable cyclic import
var (
	console = js.Global().Get("console")
)

func makeLogMessage(severity string, format string, a ...any) string {
	_, file, line, ok := runtime.Caller(2)
	// we should really be using a mutex here...
	if !ok {
		file = "???"
		line = 0
	}

	prefix := fmt.Sprintf("[go] %s: %s:%d: ", severity, file, line)
	suffix := fmt.Sprintf(format, a...)
	return prefix + suffix
}

func Error(format string, a ...any) {
	msg := makeLogMessage("ERROR", format, a...)
	console.Call("error", msg)
}

func Warn(format string, a ...any) {
	msg := makeLogMessage("WARN", format, a...)
	console.Call("warn", msg)
}

func Info(format string, a ...any) {
	msg := makeLogMessage("INFO", format, a...)
	console.Call("info", msg)
}

func Debug(format string, a ...any) {
	msg := makeLogMessage("DEBUG", format, a...)
	// too lazy to configure my console : )
	// console.Call("debug", msg)
	console.Call("log", msg)
}
