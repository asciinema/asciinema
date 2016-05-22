package util

import (
	"bufio"
	"os"
)

func ReadLine() (string, error) {
	return bufio.NewReader(os.Stdin).ReadString('\n')
}
