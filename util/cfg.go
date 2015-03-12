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
	DefaultAPIURL  = "https://asciinema.org"
	DefaultCommand = "/bin/sh"
)

type ConfigAPI struct {
	Token string
	URL   string
}

type ConfigRecord struct {
	Command string
	MaxWait uint
	Yes     bool
}

type ConfigUser struct {
	Token string
}

type ConfigFile struct {
	API    ConfigAPI
	Record ConfigRecord
	User   ConfigUser // old location of token
}

type Config struct {
	Cfg *ConfigFile
	Env map[string]string
}

func (c *Config) ApiUrl() string {
	return FirstNonBlank(c.Env["ASCIINEMA_API_URL"], c.Cfg.API.URL, DefaultAPIURL)
}

func (c *Config) ApiToken() string {
	return FirstNonBlank(c.Cfg.API.Token, c.Cfg.User.Token)
}

func (c *Config) RecordCommand() string {
	return FirstNonBlank(c.Cfg.Record.Command, c.Env["SHELL"], DefaultCommand)
}

func (c *Config) RecordMaxWait() uint {
	return c.Cfg.Record.MaxWait
}

func (c *Config) RecordYes() bool {
	return c.Cfg.Record.Yes
}

func LoadConfig(env map[string]string) (*Config, error) {
	cfg, err := loadConfig(env)
	if err != nil {
		return nil, err
	}

	return &Config{cfg, env}, nil
}

func loadConfig(env map[string]string) (*ConfigFile, error) {
	homeDir := env["HOME"]
	if homeDir == "" {
		return nil, errors.New("Need $HOME")
	}

	cfgPath := filepath.Join(homeDir, ".asciinema", "config")

	cfg, err := loadConfigFile(cfgPath)
	if err != nil {
		return nil, err
	}

	return cfg, nil
}

func loadConfigFile(cfgPath string) (*ConfigFile, error) {
	if _, err := os.Stat(cfgPath); os.IsNotExist(err) {
		if err = createConfigFile(cfgPath); err != nil {
			return nil, err
		}
	}

	var cfg ConfigFile
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
