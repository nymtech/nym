// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"fmt"
	"runtime"
	"syscall/js"
)

func makeLogMessage(severity string, format string, a ...any) string {
	_, file, line, ok := runtime.Caller(3)
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
	js.Global().Get("console").Call("error", msg)
}

func Warn(format string, a ...any) {
	msg := makeLogMessage("WARN", format, a...)
	js.Global().Get("console").Call("warn", msg)

}

func Info(format string, a ...any) {
	msg := makeLogMessage("INFO", format, a...)
	js.Global().Get("console").Call("info", msg)
}

func Debug(format string, a ...any) {
	msg := makeLogMessage("DEBUG", format, a...)
	js.Global().Get("console").Call("debug", msg)
}
