package commands

import (
	"bufio"
	"bytes"
	"flag"
	"fmt"
	"io"
	"os"

	"github.com/asciinema/asciinema-cli/api"
	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/terminal"
	"github.com/asciinema/asciinema-cli/util"
)

func Record(flags *flag.FlagSet, cfg *util.Config) cli.Command {
	command := RecordCommand{}

	flags.StringVar(
		&command.Command,
		"c",
		defaultRecCommand(cfg.Record.Command),
		"command to record, defaults to $SHELL",
	)

	flags.StringVar(
		&command.Title,
		"t",
		"",
		"set title of the asciicast",
	)

	flags.BoolVar(
		&command.NoConfirm,
		"y",
		false,
		"don't ask for upload confirmation",
	)

	return &command
}

type RecordCommand struct {
	Command   string
	Title     string
	NoConfirm bool
	Terminal  terminal.Terminal
	Api       api.Api
}

func (c *RecordCommand) Execute(args []string) error {
	rows, cols, _ := c.Terminal.Size()
	if rows > 30 || cols > 120 {
		util.Warningf("Current terminal size is %vx%v.", cols, rows)
		util.Warningf("It may be too big to be properly replayed on smaller screens.")
		util.Warningf("You can now resize it. Press <Enter> to start recording.")
		bufio.NewReader(os.Stdin).ReadString('\n')
	}

	util.Printf("Asciicast recording started.")
	util.Printf("Hit ctrl-d or type \"exit\" to finish.")

	stdout := &StdoutStream{}

	err := c.Terminal.Record(c.Command, stdout)
	if err != nil {
		return err
	}

	util.Printf("Asciicast recording finished.")

	// TODO: ask for upload confirmation

	rows, cols, _ = c.Terminal.Size()
	asciicast := api.NewAsciicast(c.Command, c.Title, rows, cols, stdout.Reader())

	url, err := c.Api.CreateAsciicast(asciicast)
	if err != nil {
		return err
	}

	fmt.Println(url)

	return nil
}

func defaultRecCommand(recCommand string) string {
	if recCommand == "" {
		recCommand = os.Getenv("SHELL")

		if recCommand == "" {
			recCommand = "/bin/sh"
		}
	}

	return recCommand
}

type StdoutStream struct {
	data []byte
}

func (s *StdoutStream) Write(p []byte) (int, error) {
	s.data = append(s.data, p...)
	return len(p), nil
}

func (s *StdoutStream) Reader() io.Reader {
	return bytes.NewReader(s.data)
}
