// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package log

import (
	"fmt"
	"runtime"
	"strings"
	"syscall/js"
)

type LogLevel = int

const (
	DISABLED LogLevel = iota
	ERROR
	WARN
	INFO
	DEBUG
	TRACE
)

var (
	// thanks to the amazing capabilities of this awesome language,
	// we couldn't put it in the `jstypes` alongside other globals
	// as it created an unresolvable cyclic import
	console = js.Global().Get("console")

	GlobalLogLevel = INFO
)

func SetLoggingLevel(raw string) {
	switch strings.ToLower(raw) {
	case "disabled":
		GlobalLogLevel = DISABLED
	case "error":
		GlobalLogLevel = ERROR
	case "warn", "warning":
		GlobalLogLevel = WARN
	case "info":
		GlobalLogLevel = INFO
	case "debug":
		GlobalLogLevel = DEBUG
	case "trace":
		GlobalLogLevel = TRACE
	default:
		Warn("\"%s\" is not a valid logging level", raw)
	}
}

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
	if GlobalLogLevel >= ERROR {
		msg := makeLogMessage("ERROR", format, a...)
		console.Call("error", msg)
	}
}

func Warn(format string, a ...any) {
	if GlobalLogLevel >= WARN {
		msg := makeLogMessage("WARN", format, a...)
		console.Call("warn", msg)
	}
}

func Info(format string, a ...any) {
	if GlobalLogLevel >= INFO {
		msg := makeLogMessage("INFO", format, a...)
		console.Call("info", msg)
	}
}

func Debug(format string, a ...any) {
	if GlobalLogLevel >= DEBUG {
		msg := makeLogMessage("DEBUG", format, a...)
		// too lazy to configure my console : )
		// console.Call("debug", msg)
		console.Call("log", msg)
	}
}

func Trace(format string, a ...any) {
	if GlobalLogLevel >= TRACE {
		msg := makeLogMessage("TRACE", format, a...)
		console.Call("log", msg)
	}
}
