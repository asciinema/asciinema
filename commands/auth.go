package commands

import (
	"fmt"

	"github.com/asciinema/asciinema/api"
)

type AuthCommand struct {
	api api.API
}

func NewAuthCommand(api api.API) *AuthCommand {
	return &AuthCommand{api}
}

func (c *AuthCommand) Execute() error {
	fmt.Println("Open the following URL in a browser to register your API token and assign any recorded asciicasts to your profile:")
	fmt.Println(c.api.AuthUrl())

	return nil
}
