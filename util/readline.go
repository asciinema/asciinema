package util

import (
	"bufio"
	"os"
)

func ReadLine() string {
	s, _ := bufio.NewReader(os.Stdin).ReadString('\n')
	return s
}
