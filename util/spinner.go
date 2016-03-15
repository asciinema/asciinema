package util

import (
	"fmt"
	"time"

	"github.com/mattn/go-colorable"
)

var (
	spinner = []rune("▉▊▋▌▍▎▏▎▍▌▋▊▉")
	stdout  = colorable.NewColorableStderr()
)

func WithSpinner(delay int, f func()) {
	stopChan := make(chan struct{})

	go func() {
		select {
		case <-stopChan:
			return
		case <-time.After(time.Duration(delay) * time.Millisecond):
		}

		i := 0
		fmt.Fprintf(stdout, "\x1b[?25l") // hide cursor

		for {
			select {
			case <-stopChan:
				return
			case <-time.After(100 * time.Millisecond):
				fmt.Fprintf(stdout, "\r%c", spinner[i])
				i = (i + 1) % len(spinner)
			}
		}
	}()

	f()

	close(stopChan)
	fmt.Fprintf(stdout, "\r\x1b[K\x1b[?25h") // clear line and show cursor back
}
