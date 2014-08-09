package util

import "syscall"

func FD_SET(p *syscall.FdSet, fd int) {
	p.Bits[fd/64] |= 1 << uint(fd) % 64
}

func FD_ISSET(p *syscall.FdSet, fd int) bool {
	return (p.Bits[fd/64] & (1 << uint(fd) % 64)) != 0
}
