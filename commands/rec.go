package commands

import (
	"fmt"
	"io/ioutil"
	"os"

	"github.com/asciinema/asciinema/api"
	"github.com/asciinema/asciinema/asciicast"
	"github.com/asciinema/asciinema/util"
)

type RecordCommand struct {
	Cfg      *util.Config
	API      api.API
	Recorder asciicast.Recorder
}

func NewRecordCommand(api api.API, cfg *util.Config) *RecordCommand {
	return &RecordCommand{
		API:      api,
		Cfg:      cfg,
		Recorder: asciicast.NewRecorder(),
	}
}

func (c *RecordCommand) Execute(command, title string, assumeYes bool, maxWait uint, filename string) error {
	var upload bool
	var err error

	if filename != "" {
		upload = false
	} else {
		filename, err = tmpPath()
		if err != nil {
			return err
		}
		upload = true
	}

	err = c.Recorder.Record(filename, command, title, maxWait, assumeYes)
	if err != nil {
		return err
	}

	if upload {
		if !assumeYes {
			util.Printf("Press <Enter> to upload, <Ctrl-C> to cancel.")
			util.ReadLine()
		}

		url, warn, err := c.API.UploadAsciicast(filename)

		if warn != "" {
			util.Warningf(warn)
		}

		if err != nil {
			util.Warningf("Upload failed, asciicast saved at %v", filename)
			util.Warningf("Retry later by executing: asciinema upload %v", filename)
			return err
		}

		os.Remove(filename)
		fmt.Println(url)
	}

	return nil
}

func tmpPath() (string, error) {
	file, err := ioutil.TempFile("", "asciicast-")
	if err != nil {
		return "", err
	}
	defer file.Close()

	return file.Name(), nil
}
