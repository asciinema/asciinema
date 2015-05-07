package asciicast

import (
	"time"

	"github.com/asciinema/asciinema/terminal"
)

const playbackDefaultMaxWait = 3600.0

type Player interface {
	Play(string, uint) error
}

type AsciicastPlayer struct {
	Terminal terminal.Terminal
}

func NewPlayer() Player {
	return &AsciicastPlayer{Terminal: terminal.NewTerminal()}
}

func (r *AsciicastPlayer) Play(path string, maxWait uint) error {
	asciicast, err := Load(path)
	if err != nil {
		return err
	}

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
