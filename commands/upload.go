package commands

import (
	"fmt"

	"github.com/asciinema/asciinema/api"
	"github.com/asciinema/asciinema/util"
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
	url, warn, err := c.API.UploadAsciicast(filename)

	if warn != "" {
		util.Warningf(warn)
	}

	if err != nil {
		return err
	}

	fmt.Println(url)

	return nil
}
