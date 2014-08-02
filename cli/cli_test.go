package cli_test

import (
	"errors"
	"flag"
	"testing"

	"github.com/asciinema/asciinema-cli/cli"
	"github.com/asciinema/asciinema-cli/util"
)

type testCommand struct {
	err    error
	called bool
}

func (c *testCommand) Execute(args []string) error {
	c.called = true
	return c.err
}

var verCmd = &testCommand{}
var fooCmd = &testCommand{}
var barCmd = &testCommand{err: errors.New("oops")}
var helpCmd = &testCommand{}

func versionCmdBuilder(*flag.FlagSet, *util.Config) cli.Command {
	return verCmd
}

func fooCmdBuilder(*flag.FlagSet, *util.Config) cli.Command {
	return fooCmd
}

func barCmdBuilder(*flag.FlagSet, *util.Config) cli.Command {
	return barCmd
}

type testConfigLoader struct{}

func (l *testConfigLoader) LoadConfig() (*util.Config, error) {
	return &util.Config{}, nil
}

func TestCLI_Run(t *testing.T) {
	commands := map[string]cli.CommandBuilderFunc{
		"version": versionCmdBuilder,
		"foo":     fooCmdBuilder,
		"bar":     barCmdBuilder,
	}

	var tests = []struct {
		args             []string
		expectedExitCode int
		expectedCommand  cli.Command
	}{
		{[]string{}, 1, helpCmd},
		{[]string{"-h"}, 0, helpCmd},
		{[]string{"wow", "-v"}, 0, verCmd},
		{[]string{"version"}, 0, verCmd},
		{[]string{"foo"}, 0, fooCmd},
		{[]string{"bar"}, 2, barCmd},
		{[]string{"nope"}, 1, helpCmd},
	}

	for _, test := range tests {
		cmd := test.expectedCommand.(*testCommand)
		cmd.called = false

		cli := &cli.CLI{
			Commands:     commands,
			HelpCommand:  helpCmd,
			ConfigLoader: &testConfigLoader{},
		}

		exitCode := cli.Run(test.args)

		if exitCode != test.expectedExitCode {
			t.Errorf("expected exit code %v for %v, got %v", test.expectedExitCode, test, exitCode)
		}

		if !cmd.called {
			t.Errorf("expected command %v to be called", test.expectedCommand)
		}
	}
}
