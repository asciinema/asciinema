package util

import "syscall"

func FD_SET(p *syscall.FdSet, fd int) {
	p.Bits[fd/32] |= 1 << uint(fd) % 32
}

func FD_ISSET(p *syscall.FdSet, fd int) bool {
	return (p.Bits[fd/32] & (1 << uint(fd) % 32)) != 0
}
