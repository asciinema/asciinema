package commands

import (
	"fmt"

	"github.com/asciinema/asciinema-cli/util"
)

type AuthCommand struct {
	apiURL   string
	apiToken string
}

func NewAuthCommand(cfg *util.Config) *AuthCommand {
	return &AuthCommand{
		apiURL:   cfg.API.URL,
		apiToken: cfg.API.Token,
	}
}

func (c *AuthCommand) Execute() error {
	fmt.Println("Open the following URL in your browser to register your API token and assign any recorded asciicasts to your profile:")
	fmt.Printf("%v/connect/%v\n", c.apiURL, c.apiToken)

	return nil
}
