package asciicast

import (
	"time"

	"github.com/asciinema/asciinema/terminal"
)

type Player interface {
	Play(*Asciicast, float64) error
}

type AsciicastPlayer struct {
	Terminal terminal.Terminal
}

func NewPlayer() Player {
	return &AsciicastPlayer{Terminal: terminal.NewTerminal()}
}

func (r *AsciicastPlayer) Play(asciicast *Asciicast, maxWait float64) error {
	for _, frame := range asciicast.Stdout {
		delay := frame.Delay
		if maxWait > 0 && delay > maxWait {
			delay = maxWait
		}
		time.Sleep(time.Duration(float64(time.Second) * delay))
		r.Terminal.Write(frame.Data)
	}

	return nil
}
