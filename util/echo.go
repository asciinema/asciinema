package util

import (
	"fmt"
	"io"
	"io/ioutil"
	"os"
)

var loggerOutput io.Writer = os.Stdout

func BeQuiet() {
	loggerOutput = ioutil.Discard
}

func Printf(s string, args ...interface{}) {
	fmt.Fprintf(loggerOutput, "\x1b[32m~ %v\x1b[0m\n", fmt.Sprintf(s, args...))
}

func ReplaceWarningf(s string, args ...interface{}) {
	fmt.Fprintf(loggerOutput, "\r\x1b[33m~ %v\x1b[0m", fmt.Sprintf(s, args...))
}

func Warningf(s string, args ...interface{}) {
	fmt.Fprintf(loggerOutput, "\x1b[33m~ %v\x1b[0m\n", fmt.Sprintf(s, args...))
}
