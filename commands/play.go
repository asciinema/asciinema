package commands

import (
	"errors"
	"flag"

	"github.com/asciinema/asciinema-cli/asciicast"
	"github.com/asciinema/asciinema-cli/cli"
)

type PlayCommand struct {
	Player asciicast.Player
}

func NewPlayCommand() cli.Command {
	return &PlayCommand{
		Player: asciicast.NewPlayer(),
	}
}

func (c *PlayCommand) RegisterFlags(flags *flag.FlagSet) {
}

func (c *PlayCommand) Execute(args []string) error {
	if len(args) == 0 {
		return errors.New("filename required. Usage: asciinema play <file>")
	}

	err := c.Player.Play(args[0])
	if err != nil {
		return err
	}

	return nil
}
