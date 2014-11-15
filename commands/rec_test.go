package commands_test

import (
	"errors"
	"io"
	"testing"
	"time"

	"github.com/asciinema/asciinema-cli/api"
	"github.com/asciinema/asciinema-cli/commands"
)

type testTerminal struct {
	err error
}

func (t *testTerminal) Size() (int, int, error) {
	return 15, 40, nil
}

func (t *testTerminal) Record(command string, stdoutCopy io.Writer) error {
	if t.err != nil {
		return t.err
	}

	stdoutCopy.Write([]byte("hello"))
	stdoutCopy.Write([]byte("world"))

	return nil
}

type testAPI struct {
	err error
	t   *testing.T
}

func (a *testAPI) CreateAsciicast(frames []api.Frame, duration time.Duration, cols, rows int, command, title string) (string, error) {
	if command != "ls" {
		a.t.Errorf("expected command to be set on asciicast")
	}

	if title != "listing" {
		a.t.Errorf("expected title to be set on asciicast")
	}

	if rows != 15 {
		a.t.Errorf("expected rows to be set on asciicast")
	}

	if cols != 40 {
		a.t.Errorf("expected cols to be set on asciicast")
	}

	stdout := string(frames[0].Data)
	if stdout != "hello" {
		a.t.Errorf(`expected frame data "%v", got "%v"`, "hello", stdout)
	}

	stdout = string(frames[1].Data)
	if stdout != "world" {
		a.t.Errorf(`expected frame data "%v", got "%v"`, "world", stdout)
	}

	if a.err != nil {
		return "", a.err
	}

	return "http://the/url", nil
}

func TestRecordCommand_Execute(t *testing.T) {
	recErr := errors.New("can't record")
	apiErr := errors.New("can't upload")

	var tests = []struct {
		recordError   error
		apiError      error
		expectedError error
	}{
		{nil, nil, nil},
		{recErr, nil, recErr},
		{nil, apiErr, apiErr},
	}

	for _, test := range tests {
		terminal := &testTerminal{err: test.recordError}
		api := &testAPI{err: test.apiError, t: t}

		command := &commands.RecordCommand{
			Command:  "ls",
			Title:    "listing",
			Terminal: terminal,
			API:      api,
		}

		err := command.Execute(nil)
		if err != test.expectedError {
			t.Errorf("expected error %v, got %v", test.expectedError, err)
		}
	}
}
