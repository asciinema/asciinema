package api

import (
	"io"
	"os"
)

type Api interface {
	CreateAsciicast(*Asciicast) (string, error)
}

func New(url, token string) *AsciinemaApi {
	return &AsciinemaApi{
		url:   url,
		token: token,
	}
}

type AsciinemaApi struct {
	url   string
	token string
}

func (a *AsciinemaApi) CreateAsciicast(asciicast *Asciicast) (string, error) {
	return "/foo", nil
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
