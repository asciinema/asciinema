package util

import (
	"errors"
	"fmt"
	"io/ioutil"
	"os"
	"path"
	"path/filepath"

	"github.com/asciinema/asciinema/Godeps/_workspace/src/code.google.com/p/gcfg"
)

const (
	DefaultAPIURL = "https://asciinema.org"
)

type Config struct {
	API struct {
		Token string
		URL   string
	}
	Record struct {
		Command string
	}
}

func LoadConfig() (*Config, error) {
	homeDir := os.Getenv("HOME")
	if homeDir == "" {
		return nil, errors.New("Need $HOME")
	}

	cfgPath := filepath.Join(homeDir, ".asciinema", "config")

	cfg, err := loadConfigFile(cfgPath)
	if err != nil {
		return nil, err
	}

	cfg.API.URL = FirstNonBlank(os.Getenv("ASCIINEMA_API_URL"), cfg.API.URL, DefaultAPIURL)

	return cfg, nil
}

func loadConfigFile(cfgPath string) (*Config, error) {
	if _, err := os.Stat(cfgPath); os.IsNotExist(err) {
		if err = createConfigFile(cfgPath); err != nil {
			return nil, err
		}
	}

	var cfg Config
	if err := gcfg.ReadFileInto(&cfg, cfgPath); err != nil {
		return nil, err
	}

	return &cfg, nil
}

func createConfigFile(cfgPath string) error {
	apiToken := NewUUID().String()
	contents := fmt.Sprintf("[api]\ntoken = %v\n", apiToken)
	os.MkdirAll(path.Dir(cfgPath), 0755)
	return ioutil.WriteFile(cfgPath, []byte(contents), 0644)
}
