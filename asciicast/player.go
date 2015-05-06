package asciicast

import (
	"math"
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

	adjustedMaxWait := float64(maxWait)
	if adjustedMaxWait <= 0 {
		adjustedMaxWait = playbackDefaultMaxWait
	}

	for _, frame := range asciicast.Stdout {
		delay := time.Duration(float64(time.Second) * math.Min(adjustedMaxWait, frame.Delay))
		time.Sleep(delay)
		r.Terminal.Write(frame.Data)
	}

	return nil
}
