package main

import (
	"fmt"
	"os"

	"github.com/asciinema/asciinema-cli/api"
	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/commands"
	"github.com/asciinema/asciinema-cli/util"
)

func main() {
	cfg, err := util.LoadConfig()
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}

	api := api.New(cfg.API.URL, cfg.API.Token, Version)

	cli := &cli.CLI{
		Commands: map[string]cli.Command{
			"rec":     commands.NewRecordCommand(api, cfg),
			"auth":    commands.NewAuthCommand(cfg),
			"version": commands.NewVersionCommand(Version, GitCommit),
		},
		HelpFunc: help,
	}

	os.Exit(cli.Run(os.Args[1:]))
}

func help() {
	fmt.Println(`usage: asciinema [-h] [-v] <command> [command-options]

Record and share your terminal sessions, the right way.

Commands:
   rec            Record terminal session
   auth           Assign local API token to asciinema.org account

Options:
   -h, --help     Display help message
   -v, --version  Display version information

   Run "asciinema <command> -h" to see the options available for the given command.`)
}
