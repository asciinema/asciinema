package util

import "syscall"

func FD_SET(p *syscall.FdSet, fd int) {
	p.X__fds_bits[fd/64] |= 1 << uint(fd) % 64
}

func FD_ISSET(p *syscall.FdSet, fd int) bool {
	return (p.X__fds_bits[fd/64] & (1 << uint(fd) % 64)) != 0
}
