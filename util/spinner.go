package util

import (
	"fmt"
	"os"
	"time"
)

var spinner = []rune("▉▊▋▌▍▎▏▎▍▌▋▊▉")

func WithSpinner(delay int, f func()) {
	stopChan := make(chan struct{})

	go func() {
		<-time.After(time.Duration(delay) * time.Millisecond)

		i := 0
		fmt.Fprintf(os.Stdout, "\x1b[?25l") // hide cursor

		for {
			select {
			case <-stopChan:
				return
			case <-time.After(100 * time.Millisecond):
				fmt.Fprintf(os.Stdout, "\r%c", spinner[i])
				i = (i + 1) % len(spinner)
			}
		}
	}()

	f()

	close(stopChan)
	fmt.Fprintf(os.Stdout, "\r\x1b[K\x1b[?25h") // clear line and show cursor back
}
