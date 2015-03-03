package commands

import (
	"fmt"

	"github.com/asciinema/asciinema-cli/api"
)

type UploadCommand struct {
	API api.API
}

func NewUploadCommand(api api.API) *UploadCommand {
	return &UploadCommand{
		API: api,
	}
}

func (c *UploadCommand) Execute(filename string) error {
	url, err := c.API.UploadAsciicast(filename)
	if err != nil {
		return err
	}

	fmt.Println(url)

	return nil
}
