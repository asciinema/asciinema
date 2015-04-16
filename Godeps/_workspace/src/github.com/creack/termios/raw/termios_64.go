// +build darwin

package raw

// Termios holds the TTY attributes. See man termios(4).
// Tested on darwin/386, darwin/amd64. See termios_32.go for others.
type Termios struct {
	Iflag  uint64
	Oflag  uint64
	Cflag  uint64
	Lflag  uint64
	Cc     [20]byte
	Ispeed uint64
	Ospeed uint64
}
