package asciicast

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"

	"github.com/asciinema/asciinema/terminal"
	"github.com/asciinema/asciinema/util"
)

const (
	warnCols = 120
	warnRows = 30
)

type Recorder interface {
	Record(string, string, string, float64, bool, map[string]string) error
}

type AsciicastRecorder struct {
	Terminal terminal.Terminal
}

func NewRecorder() Recorder {
	return &AsciicastRecorder{Terminal: terminal.NewTerminal()}
}

func (r *AsciicastRecorder) checkTerminalSize() chan<- bool {
	rows, cols, _ := r.Terminal.Size()
	doneChan := make(chan bool)
	go func() {
		winch := make(chan os.Signal, 1)
		signal.Notify(winch, syscall.SIGWINCH)

		defer signal.Stop(winch)
		defer close(winch)
		defer close(doneChan)

		for {
			select {
			case <-winch:
				newRows, newCols, _ := r.Terminal.Size()
				if cols != newCols || rows != newRows {
					cols, rows = newCols, newRows
					currentSize := fmt.Sprintf("%dx%d", cols, rows)
					util.ReplaceWarningf("Current terminal size is %s.", currentSize)
				}
			case <-doneChan:
				return
			}
		}
	}()
	return doneChan
}

func (r *AsciicastRecorder) Record(path, command, title string, maxWait float64, assumeYes bool, env map[string]string) error {
	// TODO: touch savePath to ensure writing is possible

	rows, cols, _ := r.Terminal.Size()
	if rows > warnRows || cols > warnCols {
		if !assumeYes {
			doneChan := r.checkTerminalSize()
			util.Warningf("Current terminal size is %vx%v.", cols, rows)
			util.Warningf("It may be too big to be properly replayed on smaller screens.")
			util.Warningf("You can now resize it. Press <Enter> to start recording.")
			util.ReadLine()
			doneChan <- true
		}
	}

	util.Printf("Asciicast recording started.")
	util.Printf(`Hit Ctrl-D or type "exit" to finish.`)

	stdout := NewStream(maxWait)

	err := r.Terminal.Record(command, stdout)
	if err != nil {
		return err
	}

	stdout.Close()

	util.Printf("Asciicast recording finished.")

	rows, cols, _ = r.Terminal.Size()

	asciicast := NewAsciicast(
		cols,
		rows,
		stdout.Duration().Seconds(),
		command,
		title,
		stdout.Frames,
		env,
	)

	err = Save(asciicast, path)
	if err != nil {
		return err
	}

	return nil
}
