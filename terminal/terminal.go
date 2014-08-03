package terminal

import "io"

type Terminal interface {
	Size() (int, int, error)
	Record(string, io.Writer) error
}
