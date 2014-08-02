package main

import (
	"os"

	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/commands"
	"github.com/asciinema/asciinema-cli/util"
)

func main() {
	cli := &cli.CLI{
		Commands: map[string]cli.CommandBuilderFunc{
			"auth":    commands.Auth,
			"version": commands.Version,
		},
		HelpCommand:  &commands.HelpCommand{},
		ConfigLoader: &util.FileConfigLoader{},
	}

	os.Exit(cli.Run(os.Args[1:]))
}
