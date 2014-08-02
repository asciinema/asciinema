package util

import (
	"fmt"
	"io/ioutil"
	"os"
	"os/user"

	"code.google.com/p/gcfg"
)

const (
	DEFAULT_API_URL = "https://asciinema.org"
)

type Config struct {
	Api struct {
		Token string
		Url   string
	}
	Record struct {
		Command string
	}
}

type ConfigLoader interface {
	LoadConfig() (*Config, error)
}

type FileConfigLoader struct{}

func (l *FileConfigLoader) LoadConfig() (*Config, error) {
	usr, _ := user.Current()
	path := usr.HomeDir + "/.asciinema/config"

	cfg, err := loadConfigFile(path)
	if err != nil {
		return nil, err
	}

	if cfg.Api.Url == "" {
		cfg.Api.Url = DEFAULT_API_URL
	}

	if envApiUrl := os.Getenv("ASCIINEMA_API_URL"); envApiUrl != "" {
		cfg.Api.Url = envApiUrl
	}

	return cfg, nil
}

func loadConfigFile(path string) (*Config, error) {
	if _, err := os.Stat(path); os.IsNotExist(err) {
		if err = createConfigFile(path); err != nil {
			return nil, err
		}
	}

	var cfg Config
	if err := gcfg.ReadFileInto(&cfg, path); err != nil {
		return nil, err
	}

	return &cfg, nil
}

func createConfigFile(path string) error {
	apiToken := NewUUID().String()
	contents := fmt.Sprintf("[api]\ntoken = %v\n", apiToken)
	return ioutil.WriteFile(path, []byte(contents), 0644)
}
