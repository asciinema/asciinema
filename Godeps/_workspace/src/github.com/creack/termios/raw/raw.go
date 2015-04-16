package raw

import (
	"syscall"
	"unsafe"
)

// TcSetAttr restores the terminal connected to the given file descriptor to a
// previous state.
func TcSetAttr(fd uintptr, termios *Termios) error {
	if _, _, err := syscall.Syscall(syscall.SYS_IOCTL, fd, uintptr(setTermios), uintptr(unsafe.Pointer(termios))); err != 0 {
		return err
	}
	return nil
}

// TcGetAttr retrieves the current terminal settings and returns it.
func TcGetAttr(fd uintptr) (*Termios, error) {
	var termios = &Termios{}
	if _, _, err := syscall.Syscall(syscall.SYS_IOCTL, fd, getTermios, uintptr(unsafe.Pointer(termios))); err != 0 {
		return nil, err
	}
	return termios, nil
}

// CfMakeRaw sets the flags stored in the termios structure to a state disabling
// all input and output processing, giving a ``raw I/O path''.
//
// From man cfmakeraw(3) on linux:
// termios_p->c_iflag &= ~(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
// termios_p->c_oflag &= ~OPOST;
// termios_p->c_lflag &= ~(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
// termios_p->c_cflag &= ~(CSIZE | PARENB);
// termios_p->c_cflag |= CS8;
//
func CfMakeRaw(termios *Termios) {
	termios.Iflag &^= (syscall.IGNBRK | syscall.BRKINT | syscall.PARMRK | syscall.ISTRIP | syscall.INLCR | syscall.IGNCR | syscall.ICRNL | syscall.IXON)
	termios.Oflag &^= syscall.OPOST
	termios.Lflag &^= (syscall.ECHO | syscall.ECHONL | syscall.ICANON | syscall.ISIG | syscall.IEXTEN)
	termios.Cflag &^= (syscall.CSIZE | syscall.PARENB)
	termios.Cflag |= syscall.CS8
	termios.Cc[syscall.VMIN] = 1
	termios.Cc[syscall.VTIME] = 0
}

// MakeRaw sets the flags stored in the termios structure for the given terminal fd
// to a state disabling all input and output processing, giving a ``raw I/O path''.
// It returns the current terminal's termios struct to allow to revert with TcSetAttr
func MakeRaw(fd uintptr) (*Termios, error) {
	old, err := TcGetAttr(fd)
	if err != nil {
		return nil, err
	}

	new := *old
	CfMakeRaw(&new)

	if err := TcSetAttr(fd, &new); err != nil {
		return nil, err
	}
	return old, nil
}
