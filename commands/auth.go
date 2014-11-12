package commands

import (
	"flag"
	"fmt"

	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/util"
)

type AuthCommand struct {
	apiUrl   string
	apiToken string
}

func NewAuthCommand(cfg *util.Config) cli.Command {
	return &AuthCommand{
		apiUrl:   cfg.Api.Url,
		apiToken: cfg.Api.Token,
	}
}

func (c *AuthCommand) RegisterFlags(flags *flag.FlagSet) {
}

func (c *AuthCommand) Execute(args []string) error {
	fmt.Println("Open the following URL in your browser to register your API token and assign any recorded asciicasts to your profile:")
	fmt.Printf("%v/connect/%v\n", c.apiUrl, c.apiToken)

	return nil
}
