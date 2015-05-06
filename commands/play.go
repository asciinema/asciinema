package commands

import "github.com/asciinema/asciinema/asciicast"

type PlayCommand struct {
	Player asciicast.Player
}

func NewPlayCommand() *PlayCommand {
	return &PlayCommand{
		Player: asciicast.NewPlayer(),
	}
}

func (c *PlayCommand) Execute(filename string, maxWait uint) error {
	return c.Player.Play(filename, float64(maxWait))
}
