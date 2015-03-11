package asciicast

import (
	"github.com/asciinema/asciinema/terminal"
	"github.com/asciinema/asciinema/util"
)

type Recorder interface {
	Record(string, string, string, uint, bool, map[string]string) error
}

type AsciicastRecorder struct {
	Terminal terminal.Terminal
}

func NewRecorder() Recorder {
	return &AsciicastRecorder{Terminal: terminal.NewTerminal()}
}

func (r *AsciicastRecorder) Record(path, command, title string, maxWait uint, assumeYes bool, env map[string]string) error {
	// TODO: touch savePath to ensure writing is possible

	rows, cols, _ := r.Terminal.Size()
	if rows > 30 || cols > 120 {
		util.Warningf("Current terminal size is %vx%v.", cols, rows)
		util.Warningf("It may be too big to be properly replayed on smaller screens.")

		if !assumeYes {
			util.Warningf("You can now resize it. Press <Enter> to start recording.")
			util.ReadLine()
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
