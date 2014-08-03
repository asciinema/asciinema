package commands_test

import (
	"bytes"
	"errors"
	"io"
	"testing"

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
	return nil
}

type testApi struct {
	err error
	t   *testing.T
}

func (a *testApi) CreateAsciicast(asciicast *api.Asciicast) (string, error) {
	if asciicast.Command != "ls" {
		a.t.Errorf("expected command to be set on asciicast")
	}

	if asciicast.Title != "listing" {
		a.t.Errorf("expected title to be set on asciicast")
	}

	if asciicast.Rows != 15 {
		a.t.Errorf("expected rows to be set on asciicast")
	}

	if asciicast.Cols != 40 {
		a.t.Errorf("expected cols to be set on asciicast")
	}

	buf := new(bytes.Buffer)
	buf.ReadFrom(asciicast.Stdout)
	stdout := buf.String()
	if stdout != "hello" {
		a.t.Errorf("expected recorded stdout to be set on asciicast")
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
		api := &testApi{err: test.apiError, t: t}

		command := &commands.RecordCommand{
			Command:  "ls",
			Title:    "listing",
			Terminal: terminal,
			Api:      api,
		}

		err := command.Execute(nil)
		if err != test.expectedError {
			t.Errorf("expected error %v, got %v", test.expectedError, err)
		}
	}
}
