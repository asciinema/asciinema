package cli

import (
	"flag"

	"github.com/asciinema/asciinema-cli/util"
)

type CommandBuilderFunc func(*flag.FlagSet, *util.Config) Command

type Command interface {
	Execute([]string) error
}
