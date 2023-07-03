// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"fmt"
	"runtime"
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
	jsConsole.Call("error", msg)
}

func Warn(format string, a ...any) {
	msg := makeLogMessage("WARN", format, a...)
	jsConsole.Call("warn", msg)

}

func Info(format string, a ...any) {
	msg := makeLogMessage("INFO", format, a...)
	jsConsole.Call("info", msg)
}

func Debug(format string, a ...any) {
	msg := makeLogMessage("DEBUG", format, a...)
	// too lazy to configure my console : )
	//jsConsole.Call("debug", msg)
	jsConsole.Call("log", msg)
}
