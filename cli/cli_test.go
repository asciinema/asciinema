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

var helpCmd, verCmd, fooCmd, barCmd *testCommand

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
		"foo": fooCmdBuilder,
		"bar": barCmdBuilder,
	}

	var tests = []struct {
		args             []string
		expectedExitCode int
		expectedCommand  **testCommand
	}{
		{[]string{}, 1, &helpCmd},
		{[]string{"-h"}, 0, &helpCmd},
		{[]string{"wow", "-v"}, 0, &verCmd},
		{[]string{"version"}, 0, &verCmd},
		{[]string{"foo"}, 0, &fooCmd},
		{[]string{"bar"}, 2, &barCmd},
		{[]string{"nope"}, 1, &helpCmd},
	}

	for _, test := range tests {
		helpCmd = &testCommand{}
		verCmd = &testCommand{}
		fooCmd = &testCommand{}
		barCmd = &testCommand{err: errors.New("oops")}

		cli := &cli.CLI{
			Commands:     commands,
			HelpFunc:     func() { helpCmd.Execute(nil) },
			VersionFunc:  func() { verCmd.Execute(nil) },
			ConfigLoader: &testConfigLoader{},
		}

		exitCode := cli.Run(test.args)

		if exitCode != test.expectedExitCode {
			t.Errorf("expected exit code %v for %v, got %v", test.expectedExitCode, test, exitCode)
		}

		if !(*test.expectedCommand).called {
			t.Errorf("expected command %v to be called", test.expectedCommand)
		}
	}
}
