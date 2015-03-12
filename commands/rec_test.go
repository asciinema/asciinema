package commands_test

import (
	"errors"
	"testing"

	"github.com/asciinema/asciinema/commands"
)

type testRecorder struct {
	err error
}

func (r *testRecorder) Record(path, command, title string, maxWait uint, assumeYes bool, env map[string]string) error {
	return r.err
}

type testAPI struct {
	err error
	t   *testing.T
}

func (a *testAPI) AuthUrl() string {
	return ""
}

func (a *testAPI) UploadAsciicast(path string) (string, string, error) {
	if a.err != nil {
		return "", "", a.err
	}

	return "http://the/url", "", nil
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
		recorder := &testRecorder{err: test.recordError}
		api := &testAPI{err: test.apiError, t: t}

		command := &commands.RecordCommand{
			Recorder: recorder,
			API:      api,
		}

		err := command.Execute("ls", "listing", false, 5, "")
		if err != test.expectedError {
			t.Errorf("expected error %v, got %v", test.expectedError, err)
		}
	}
}
