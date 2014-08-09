package util

import "syscall"

func Select(nfd int, r *syscall.FdSet, w *syscall.FdSet, e *syscall.FdSet, timeout *syscall.Timeval) error {
	_, err := syscall.Select(nfd, r, w, e, timeout)
	return err
}
