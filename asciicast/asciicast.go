package asciicast

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"os"
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

func Load(path string) (*Asciicast, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	dec := json.NewDecoder(file)
	asciicast := &Asciicast{}

	err = dec.Decode(asciicast)
	if err != nil {
		return nil, err
	}

	return asciicast, nil
}
