package api

import (
	"io"
	"os"
)

type Api interface {
	CreateAsciicast(*Asciicast) (string, error)
}

type Asciicast struct {
	Command  string
	Title    string
	Rows     int
	Cols     int
	Shell    string
	Username string
	Term     string
	Stdout   io.Reader
}

func NewAsciicast(command, title string, rows, cols int, stdout io.Reader) *Asciicast {
	return &Asciicast{
		Command:  command,
		Title:    title,
		Rows:     rows,
		Cols:     cols,
		Shell:    os.Getenv("SHELL"),
		Username: os.Getenv("USER"),
		Term:     os.Getenv("TERM"),
		Stdout:   stdout,
	}
}
