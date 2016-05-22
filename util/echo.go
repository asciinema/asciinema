package util

import (
	"fmt"
	"io"
	"io/ioutil"

	"github.com/mattn/go-colorable"
)

var loggerOutput io.Writer = colorable.NewColorableStdout()

func BeQuiet() {
	loggerOutput = ioutil.Discard
}

func Printf(s string, args ...interface{}) {
	fmt.Fprintf(loggerOutput, "\x1b[32m~ %v\x1b[0m\n", fmt.Sprintf(s, args...))
}

func Warningf(s string, args ...interface{}) {
	fmt.Fprintf(loggerOutput, "\x1b[33m~ %v\x1b[0m\n", fmt.Sprintf(s, args...))
}
