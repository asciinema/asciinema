package cli

import (
	"flag"
	"fmt"

	"github.com/asciinema/asciinema-cli/util"
)

type CLI struct {
	Commands     map[string]CommandBuilderFunc
	HelpCommand  Command
	ConfigLoader util.ConfigLoader
}

func (c *CLI) Run(args []string) int {
	commandName, args := parseArgs(args)

	if commandName == "help" {
		c.HelpCommand.Execute(nil)
		return 0
	}

	commandBuilder := c.Commands[commandName]
	if commandBuilder == nil {
		c.HelpCommand.Execute(nil)
		return 1
	}

	flags := flag.NewFlagSet(commandName, flag.ExitOnError)

	config, err := c.ConfigLoader.LoadConfig()
	if err != nil {
		fmt.Println(err)
		return 1
	}

	command := commandBuilder(flags, config)
	flags.Parse(args)

	err = command.Execute(flags.Args())
	if err != nil {
		fmt.Println(err)
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

		if command == "-h" {
			command = "help"
		}
	}

	return command, args
}
