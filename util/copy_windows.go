// +build windows

package util

import (
	"bufio"
	"io"
	"os"
)

func Copy(dst io.Writer, src *os.File) func() {
	br, bw := bufio.NewReader(src), bufio.NewWriter(dst)

	go func() {
		for {
			_, err := io.Copy(bw, br)
			if err == io.EOF {
				break
			}
		}
	}()

	return func() {
		bw.Write([]byte("x"))
		src.Close()
		bw.Flush()
	}
}
