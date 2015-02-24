package commands

import (
	"flag"
	"fmt"
	"io/ioutil"
	"os"

	"github.com/asciinema/asciinema-cli/api"
	"github.com/asciinema/asciinema-cli/asciicast"
	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/util"
)

type RecordCommand struct {
	Cfg       *util.Config
	API       api.API
	Recorder  asciicast.Recorder
	Command   string
	Title     string
	NoConfirm bool
	MaxWait   uint
}

func NewRecordCommand(api api.API, cfg *util.Config) cli.Command {
	return &RecordCommand{
		API:      api,
		Cfg:      cfg,
		Recorder: asciicast.NewRecorder(),
	}
}

func (c *RecordCommand) RegisterFlags(flags *flag.FlagSet) {
	flags.StringVar(
		&c.Command,
		"c",
		defaultRecCommand(c.Cfg.Record.Command),
		"command to record",
	)

	flags.StringVar(
		&c.Title,
		"t",
		"",
		"set asciicast title",
	)

	flags.BoolVar(
		&c.NoConfirm,
		"y",
		false,
		"upload without asking for confirmation",
	)

	flags.UintVar(
		&c.MaxWait,
		"max-wait",
		0,
		"reduce recorded terminal inactivity to maximum of <max-wait> seconds (0 turns off)",
	)
}

func (c *RecordCommand) Execute(args []string) error {
	var path string
	var upload bool
	var err error

	if len(args) > 0 {
		path = args[0]
		upload = false
	} else {
		path, err = tmpPath()
		if err != nil {
			return err
		}
		upload = true
	}

	err = c.Recorder.Record(path, c.Command, c.Title, c.MaxWait)
	if err != nil {
		return err
	}

	if upload {
		if !c.NoConfirm {
			util.Printf("Press <Enter> to upload, <Ctrl-C> to cancel.")
			util.ReadLine()
		}

		url, err := c.API.UploadAsciicast(path)
		if err != nil {
			util.Warningf("Upload failed, asciicast saved at %v", path)
			util.Warningf("Retry later by executing: asciinema upload %v", path)
			return err
		}

		os.Remove(path)
		fmt.Println(url)
	}

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

func tmpPath() (string, error) {
	file, err := ioutil.TempFile("", "asciicast-")
	if err != nil {
		return "", err
	}
	defer os.Remove(file.Name())

	return file.Name(), nil
}
