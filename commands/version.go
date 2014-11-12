package commands

import (
	"flag"
	"fmt"

	"github.com/asciinema/asciinema-cli/cli"
)

type VersionCommand struct {
	version   string
	gitCommit string
}

func NewVersionCommand(version, gitCommit string) cli.Command {
	return &VersionCommand{
		version:   version,
		gitCommit: gitCommit,
	}
}

func (c *VersionCommand) RegisterFlags(flags *flag.FlagSet) {
}

func (c *VersionCommand) Execute(args []string) error {
	var commitInfo string

	if c.gitCommit != "" {
		commitInfo = fmt.Sprintf("-%v", c.gitCommit)
	}

	fmt.Printf("asciinema %v%v\n", c.version, commitInfo)

	return nil
}
