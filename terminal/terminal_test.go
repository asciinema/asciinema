package terminal_test

import (
	"testing"

	"github.com/asciinema/asciinema-cli/terminal"
)

type testWriter struct {
	chunks []string
}

func (w *testWriter) Write(p []byte) (int, error) {
	w.chunks = append(w.chunks, string(p))
	return len(p), nil
}

func TestRecord(t *testing.T) {
	command := `python -c "
import sys, time
sys.stdout.write('foo')
sys.stdout.flush()
time.sleep(0.01)
sys.stdout.write('bar')
"`
	stdoutCopy := &testWriter{}

	err := terminal.New().Record(command, stdoutCopy)

	if err != nil {
		t.Errorf("got error: %v", err)
		return
	}

	chunk := stdoutCopy.chunks[0]
	if chunk != "foo" {
		t.Errorf("expected \"foo\", got \"%v\"", chunk)
	}

	chunk = stdoutCopy.chunks[1]
	if chunk != "bar" {
		t.Errorf("expected \"bar\", got \"%v\"", chunk)
	}

	// TODO: check if ASCIINEMA_REC env var was set
}
