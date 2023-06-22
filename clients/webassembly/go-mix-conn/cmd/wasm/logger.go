// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"fmt"
	"log"
	"os"
)

var (
	ErrorLogger   *log.Logger
	WarningLogger *log.Logger
	InfoLogger    *log.Logger
	DebugLogger   *log.Logger
)

func SetupLogging() {
	ErrorLogger = log.New(os.Stderr, "ERROR: ", log.Ltime|log.Llongfile)
	WarningLogger = log.New(os.Stderr, "WARN: ", log.Ltime|log.Llongfile)
	InfoLogger = log.New(os.Stderr, "INFO: ", log.Ltime|log.Llongfile)
	DebugLogger = log.New(os.Stderr, "DEBUG: ", log.Ltime|log.Llongfile)
}

func Error(format string, a ...any) {
	if ErrorLogger != nil {
		_ = ErrorLogger.Output(3, fmt.Sprintf(format, a))
	}
}

func Warn(format string, a ...any) {
	if WarningLogger != nil {
		_ = WarningLogger.Output(3, fmt.Sprintf(format, a))
	}
}

func Info(format string, a ...any) {
	if InfoLogger != nil {
		_ = InfoLogger.Output(3, fmt.Sprintf(format, a))
	}
}

func Debug(format string, a ...any) {
	if DebugLogger != nil {
		_ = DebugLogger.Output(3, fmt.Sprintf(format, a))
	}
}
