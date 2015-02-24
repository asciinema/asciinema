package asciicast

import (
	"encoding/json"
	"io/ioutil"
	"os"
)

type Env struct {
	Term  string `json:"TERM"`
	Shell string `json:"SHELL"`
}

type Asciicast struct {
	Version  int     `json:"version"`
	Width    int     `json:"width"`
	Height   int     `json:"height"`
	Duration float64 `json:"duration"`
	Command  string  `json:"command"`
	Title    string  `json:"title"`
	Env      *Env    `json:"env"`
	Stdout   []Frame `json:"stdout"`
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
