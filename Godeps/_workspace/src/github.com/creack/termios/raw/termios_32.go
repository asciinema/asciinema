// +build linux freebsd

package raw

// Termios holds the TTY attributes. See man termios(4).
// Tested on linux386, linux/arm, linux/amd64,
//           freebsd/386, freebsd/arm, freebsd/amd64.
// See tremios_64.go for darwin.
type Termios struct {
	Iflag  uint32
	Oflag  uint32
	Cflag  uint32
	Lflag  uint32
	Cc     [20]byte
	Ispeed uint32
	Ospeed uint32
}
