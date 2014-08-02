package util

import "fmt"

func Printf(s string, args ...interface{}) {
	fmt.Printf("\x1b[32m~ %v\x1b[0m\n", fmt.Sprintf(s, args...))
}

func Warningf(s string, args ...interface{}) {
	fmt.Printf("\x1b[33m~ %v\x1b[0m\n", fmt.Sprintf(s, args...))
}
