package cli

import (
	"flag"
	"fmt"
)

type CLI struct {
	Commands map[string]Command
	HelpFunc func()
}

func (c *CLI) Run(args []string) int {
	commandName, args := parseArgs(args)

	if commandName == "help" {
		c.HelpFunc()
		return 0
	}

	command := c.Commands[commandName]
	if command == nil {
		c.HelpFunc()
		return 1
	}

	flags := flag.NewFlagSet(commandName, flag.ExitOnError)

	command.RegisterFlags(flags)
	flags.Parse(args)

	err := command.Execute(flags.Args())
	if err != nil {
		fmt.Printf("Error: %v\n", err)
		return 2
	}

	return 0
}

func parseArgs(args []string) (string, []string) {
	command := ""

	for _, arg := range args {
		if arg == "-v" || arg == "--version" {
			args = []string{"version"}
			break
		}
	}

	if len(args) > 0 {
		command = args[0]
		args = args[1:]

		if command == "-h" || command == "--help" {
			command = "help"
		}
	}

	return command, args
}
