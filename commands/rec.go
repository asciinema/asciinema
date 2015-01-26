package commands

import (
	"errors"
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
	MaxWait   uint
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
	if !util.IsUtf8Locale() {
		return errors.New("asciinema needs a UTF-8 native locale to run. Check the output of `locale` command.")
	}

	rows, cols, _ := c.Terminal.Size()
	if rows > 30 || cols > 120 {
		util.Warningf("Current terminal size is %vx%v.", cols, rows)
		util.Warningf("It may be too big to be properly replayed on smaller screens.")
		util.Warningf("You can now resize it. Press <Enter> to start recording.")
		util.ReadLine()
	}

	util.Printf("Asciicast recording started.")
	util.Printf(`Hit Ctrl-D or type "exit" to finish.`)

	stdout := NewStream(c.MaxWait)

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
	elapsedTime   time.Duration
	lastWriteTime time.Time
	maxWait       time.Duration
}

func NewStream(maxWait uint) *Stream {
	now := time.Now()

	return &Stream{
		lastWriteTime: now,
		maxWait:       time.Duration(maxWait) * time.Second,
	}
}

func (s *Stream) Write(p []byte) (int, error) {
	frame := api.Frame{}
	frame.Delay = s.incrementElapsedTime().Seconds()
	frame.Data = make([]byte, len(p))
	copy(frame.Data, p)
	s.Frames = append(s.Frames, frame)

	return len(p), nil
}

func (s *Stream) Close() {
	s.incrementElapsedTime()
}

func (s *Stream) Duration() time.Duration {
	return s.elapsedTime
}

func (s *Stream) incrementElapsedTime() time.Duration {
	now := time.Now()
	d := now.Sub(s.lastWriteTime)

	if s.maxWait > 0 && d > s.maxWait {
		d = s.maxWait
	}

	s.elapsedTime += d
	s.lastWriteTime = now

	return d
}
