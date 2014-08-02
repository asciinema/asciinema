package commands

import (
	"flag"
	"fmt"

	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/util"
)

func Version(flags *flag.FlagSet, cfg *util.Config) cli.Command {
	return &VersionCommand{}
}

type VersionCommand struct{}

func (c *VersionCommand) Execute(args []string) error {
	fmt.Println("asciinema xxx")

	return nil
}
