package cli

import "flag"

type Command interface {
	Execute([]string) error
	RegisterFlags(*flag.FlagSet)
}
