package main

import (
	"fmt"
	"os"

	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/commands"
	"github.com/asciinema/asciinema-cli/util"
)

var (
	Version string
	Commit  string
)

func main() {
	cli := &cli.CLI{
		Commands: map[string]cli.CommandBuilderFunc{
			"rec":  commands.Record,
			"auth": commands.Auth,
		},
		HelpFunc:     help,
		VersionFunc:  version,
		ConfigLoader: &util.FileConfigLoader{},
	}

	os.Exit(cli.Run(os.Args[1:]))
}

func version() {
	fmt.Printf("asciinema %v (%v)\n", Version, Commit)
}

func help() {
	fmt.Println(`usage: asciinema <command> [options]

Asciinema terminal recorder.

Commands:
   rec       Record asciicast
   auth      Assign local API token to asciinema.org account

Run "asciinema <command> -h" to see the options available for the given command.`)
}
