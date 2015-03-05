package asciicast

import (
	"time"

	"github.com/asciinema/asciinema/terminal"
)

type Player interface {
	Play(string) error
}

type AsciicastPlayer struct {
	Terminal terminal.Terminal
}

func NewPlayer() Player {
	return &AsciicastPlayer{Terminal: terminal.NewTerminal()}
}

func (r *AsciicastPlayer) Play(path string) error {
	asciicast, err := Load(path)
	if err != nil {
		return err
	}

	for _, frame := range asciicast.Stdout {
		delay := time.Duration(float64(time.Second) * frame.Delay)
		time.Sleep(delay)
		r.Terminal.Write(frame.Data)
	}

	return nil
}
