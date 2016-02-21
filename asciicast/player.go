package asciicast

import (
	"time"

	"github.com/asciinema/asciinema/terminal"
)

type Player interface {
	Play(*Asciicast, uint) error
}

type AsciicastPlayer struct {
	Terminal terminal.Terminal
}

func NewPlayer() Player {
	return &AsciicastPlayer{Terminal: terminal.NewTerminal()}
}

func (r *AsciicastPlayer) Play(asciicast *Asciicast, maxWait uint) error {
	for _, frame := range asciicast.Stdout {
		delay := frame.Delay
		if maxWait > 0 && delay > float64(maxWait) {
			delay = float64(maxWait)
		}
		time.Sleep(time.Duration(float64(time.Second) * delay))
		r.Terminal.Write(frame.Data)
	}

	return nil
}
