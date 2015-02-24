package commands

import (
	"errors"
	"flag"
	"fmt"

	"github.com/asciinema/asciinema-cli/api"
	"github.com/asciinema/asciinema-cli/cli"
)

type UploadCommand struct {
	API api.API
}

func NewUploadCommand(api api.API) cli.Command {
	return &UploadCommand{
		API: api,
	}
}

func (c *UploadCommand) RegisterFlags(flags *flag.FlagSet) {
}

func (c *UploadCommand) Execute(args []string) error {
	if len(args) == 0 {
		return errors.New("filename required. Usage: asciinema upload <file>")
	}

	url, err := c.API.UploadAsciicast(args[0])
	if err != nil {
		return err
	}

	fmt.Println(url)

	return nil
}
