package commands

import "fmt"

type HelpCommand struct{}

func (c *HelpCommand) Execute(args []string) error {
	fmt.Println(`usage: asciinema <command> [options]

Asciinema terminal recorder.

Commands:
   rec       Record asciicast
   auth      Assign local API token to asciinema.org account
   version   Show version information (also -v and --version)

Run "asciinema <command> -h" to see the options available for the given command.`)

	return nil
}
