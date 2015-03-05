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

func (c *PlayCommand) Execute(filename string) error {
	return c.Player.Play(filename)
}
