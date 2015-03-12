package commands

import (
	"fmt"

	"github.com/asciinema/asciinema/util"
)

type AuthCommand struct {
	cfg *util.Config
}

func NewAuthCommand(cfg *util.Config) *AuthCommand {
	return &AuthCommand{cfg}
}

func (c *AuthCommand) Execute() error {
	fmt.Println("Open the following URL in your browser to register your API token and assign any recorded asciicasts to your profile:")
	fmt.Printf("%v/connect/%v\n", c.cfg.ApiUrl(), c.cfg.ApiToken())

	return nil
}
