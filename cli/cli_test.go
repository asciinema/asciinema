package cli_test

import (
	"errors"
	"flag"
	"testing"

	"github.com/asciinema/asciinema-cli/cli"
)

type testCommand struct {
	err    error
	called bool
}

func (c *testCommand) Execute(args []string) error {
	c.called = true
	return c.err
}

func (c *testCommand) RegisterFlags(flags *flag.FlagSet) {
}

func (c *testCommand) reset() {
	c.called = false
}

func TestCLI_Run(t *testing.T) {
	helpCmd := &testCommand{}
	verCmd := &testCommand{}
	fooCmd := &testCommand{}
	barCmd := &testCommand{err: errors.New("oops")}

	commands := map[string]cli.Command{
		"version": verCmd,
		"foo":     fooCmd,
		"bar":     barCmd,
	}

	var tests = []struct {
		args             []string
		expectedExitCode int
		expectedCommand  *testCommand
	}{
		{[]string{}, 1, helpCmd},
		{[]string{"-h"}, 0, helpCmd},
		{[]string{"--help"}, 0, helpCmd},
		{[]string{"wow", "-v"}, 0, verCmd},
		{[]string{"version"}, 0, verCmd},
		{[]string{"foo"}, 0, fooCmd},
		{[]string{"bar"}, 2, barCmd},
		{[]string{"nope"}, 1, helpCmd},
	}

	for _, test := range tests {
		helpCmd.reset()
		verCmd.reset()
		fooCmd.reset()
		barCmd.reset()

		cli := &cli.CLI{
			Commands: commands,
			HelpFunc: func() { helpCmd.Execute(nil) },
		}

		exitCode := cli.Run(test.args)

		if exitCode != test.expectedExitCode {
			t.Errorf("expected exit code %v for %v, got %v", test.expectedExitCode, test, exitCode)
		}

		if !test.expectedCommand.called {
			t.Errorf("expected command %v to be called", test.expectedCommand)
		}
	}
}
