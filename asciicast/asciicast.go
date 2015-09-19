package asciicast

import (
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"os"
	"strings"
)

type Env struct {
	Term  string `json:"TERM"`
	Shell string `json:"SHELL"`
}

type Duration float64

func (d Duration) MarshalJSON() ([]byte, error) {
	return []byte(fmt.Sprintf(`%.6f`, d)), nil
}

type Asciicast struct {
	Version  int      `json:"version"`
	Width    int      `json:"width"`
	Height   int      `json:"height"`
	Duration Duration `json:"duration"`
	Command  string   `json:"command"`
	Title    string   `json:"title"`
	Env      *Env     `json:"env"`
	Stdout   []Frame  `json:"stdout"`
}

func NewAsciicast(width, height int, duration float64, command, title string, frames []Frame, env map[string]string) *Asciicast {
	return &Asciicast{
		Version:  1,
		Width:    width,
		Height:   height,
		Duration: Duration(duration),
		Command:  command,
		Title:    title,
		Env:      &Env{Term: env["TERM"], Shell: env["SHELL"]},
		Stdout:   frames,
	}
}

func Save(asciicast *Asciicast, path string) error {
	bytes, err := json.MarshalIndent(asciicast, "", "  ")
	if err != nil {
		return err
	}

	err = ioutil.WriteFile(path, bytes, 0644)
	if err != nil {
		return err
	}

	return nil
}

// asciinema play file.json
// asciinema play https://asciinema.org/a/123.json
// asciinema play ipfs://ipfs/QmbdpNCwqeZgnmAWBCQcs8u6Ts6P2ku97tfKAycE1XY88p
// asciinema play -

func getSource(url string) (io.ReadCloser, error) {
	if strings.HasPrefix(url, "ipfs://") {
		url = fmt.Sprintf("https://ipfs.io/%v", url[7:len(url)])
	}

	if url == "-" {
		return os.Stdin, nil
	}

	if strings.HasPrefix(url, "http://") || strings.HasPrefix(url, "https://") {
		resp, err := http.Get(url)

		if err != nil {
			return nil, err
		}

		if resp.StatusCode != 200 {
			resp.Body.Close()
			return nil, fmt.Errorf("got status %v when requesting %v", resp.StatusCode, url)
		}

		return resp.Body, nil
	}

	return os.Open(url)
}

func Load(url string) (*Asciicast, error) {
	source, err := getSource(url)
	if err != nil {
		return nil, err
	}
	defer source.Close()

	dec := json.NewDecoder(source)
	asciicast := &Asciicast{}

	if err = dec.Decode(asciicast); err != nil {
		return nil, err
	}

	return asciicast, nil
}
