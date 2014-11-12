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

	api := api.New(cfg.Api.Url, cfg.Api.Token)

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
	fmt.Println(`usage: asciinema <command> [options]

Asciinema terminal recorder.

Commands:
   rec       Record asciicast
   auth      Assign local API token to asciinema.org account

Run "asciinema <command> -h" to see the options available for the given command.`)
}
