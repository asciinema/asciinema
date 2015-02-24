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
	if !util.IsUtf8Locale() {
		fmt.Println("asciinema needs a UTF-8 native locale to run. Check the output of `locale` command.")
		os.Exit(1)
	}

	cfg, err := util.LoadConfig()
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}

	api := api.New(cfg.API.URL, cfg.API.Token, Version)

	cli := &cli.CLI{
		Commands: map[string]cli.Command{
			"rec":     commands.NewRecordCommand(api, cfg),
			"play":    commands.NewPlayCommand(),
			"upload":  commands.NewUploadCommand(api),
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
   rec [filename]         Record terminal session
   play <filename>        Replay terminal session
   upload <filename>      Upload locally saved terminal session
   auth                   Assign local API token to asciinema.org account

Options:
   -h, --help     Display help message
   -v, --version  Display version information

   Run "asciinema <command> -h" to see the options available for the given command.`)
}
