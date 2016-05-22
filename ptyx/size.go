// +build !windows

package ptyx

// Extension to github.com/kr/pty adding Setsize()

import (
	"os"
	"syscall"
	"unsafe"
)

type winsize struct {
	ws_row    uint16
	ws_col    uint16
	ws_xpixel uint16
	ws_ypixel uint16
}

func ioctl(fd, cmd, ptr uintptr) error {
	_, _, e := syscall.Syscall(syscall.SYS_IOCTL, fd, cmd, ptr)
	if e != 0 {
		return e
	}
	return nil
}

func Setsize(f *os.File, rows int, cols int) error {
	var ws winsize
	ws.ws_row = uint16(rows)
	ws.ws_col = uint16(cols)
	return ioctl(f.Fd(), syscall.TIOCSWINSZ, uintptr(unsafe.Pointer(&ws)))
}
