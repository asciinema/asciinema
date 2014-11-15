package commands

import (
	"flag"
	"fmt"
	"os"
	"time"

	"github.com/asciinema/asciinema-cli/api"
	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/terminal"
	"github.com/asciinema/asciinema-cli/util"
)

type RecordCommand struct {
	Cfg       *util.Config
	API       api.API
	Terminal  terminal.Terminal
	Command   string
	Title     string
	NoConfirm bool
}

func NewRecordCommand(api api.API, cfg *util.Config) cli.Command {
	return &RecordCommand{
		API:      api,
		Cfg:      cfg,
		Terminal: terminal.New(),
	}
}

func (c *RecordCommand) RegisterFlags(flags *flag.FlagSet) {
	flags.StringVar(
		&c.Command,
		"c",
		defaultRecCommand(c.Cfg.Record.Command),
		"command to record, defaults to $SHELL",
	)

	flags.StringVar(
		&c.Title,
		"t",
		"",
		"set title of the asciicast",
	)

	flags.BoolVar(
		&c.NoConfirm,
		"y",
		false,
		"upload without asking for confirmation",
	)
}

func (c *RecordCommand) Execute(args []string) error {
	rows, cols, _ := c.Terminal.Size()
	if rows > 30 || cols > 120 {
		util.Warningf("Current terminal size is %vx%v.", cols, rows)
		util.Warningf("It may be too big to be properly replayed on smaller screens.")
		util.Warningf("You can now resize it. Press <Enter> to start recording.")
		util.ReadLine()
	}

	util.Printf("Asciicast recording started.")
	util.Printf(`Hit ctrl-d or type "exit" to finish.`)

	stdout := NewStream()

	err := c.Terminal.Record(c.Command, stdout)
	if err != nil {
		return err
	}

	stdout.Close()

	util.Printf("Asciicast recording finished.")

	if !c.NoConfirm {
		util.Printf("Press <Enter> to upload, <Ctrl-C> to cancel.")
		util.ReadLine()
	}

	rows, cols, _ = c.Terminal.Size()

	url, err := c.API.CreateAsciicast(stdout.Frames, stdout.Duration(), cols, rows, c.Command, c.Title)
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

type Stream struct {
	Frames        []api.Frame
	startTime     time.Time
	lastWriteTime time.Time
}

func NewStream() *Stream {
	now := time.Now()

	return &Stream{
		startTime:     now,
		lastWriteTime: now,
	}
}

func (s *Stream) Write(p []byte) (int, error) {
	now := time.Now()
	frame := api.Frame{}
	frame.Delay = now.Sub(s.lastWriteTime).Seconds()
	frame.Data = make([]byte, len(p))
	copy(frame.Data, p)
	s.Frames = append(s.Frames, frame)
	s.lastWriteTime = now
	return len(p), nil
}

func (s *Stream) Close() {
	s.lastWriteTime = time.Now()
}

func (s *Stream) Duration() time.Duration {
	return s.lastWriteTime.Sub(s.startTime)
}
