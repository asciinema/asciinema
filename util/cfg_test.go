package util_test

import (
	"testing"

	"github.com/asciinema/asciinema/util"
)

func TestConfig_ApiUrl(t *testing.T) {
	var tests = []struct {
		cfg      util.ConfigFile
		env      map[string]string
		expected string
	}{
		{
			util.ConfigFile{},
			map[string]string{},
			"https://asciinema.org",
		},
		{
			util.ConfigFile{API: util.ConfigAPI{URL: "https://asciinema.example.com"}},
			map[string]string{},
			"https://asciinema.example.com",
		},
		{
			util.ConfigFile{API: util.ConfigAPI{URL: "https://asciinema.example.com"}},
			map[string]string{"ASCIINEMA_API_URL": "http://localhost:3000"},
			"http://localhost:3000",
		},
	}

	for _, test := range tests {
		cfg := util.Config{&test.cfg, test.env}
		actual := cfg.ApiUrl()

		if actual != test.expected {
			t.Errorf(`expected "%v", got "%v"`, test.expected, actual)
		}
	}
}

func TestConfig_ApiToken(t *testing.T) {
	var tests = []struct {
		cfg      util.ConfigFile
		expected string
	}{
		{
			util.ConfigFile{},
			"",
		},
		{
			util.ConfigFile{API: util.ConfigAPI{Token: "foo"}},
			"foo",
		},
		{
			util.ConfigFile{User: util.ConfigUser{Token: "foo"}},
			"foo",
		},
	}

	for _, test := range tests {
		cfg := util.Config{&test.cfg, nil}
		actual := cfg.ApiToken()

		if actual != test.expected {
			t.Errorf(`expected "%v", got "%v"`, test.expected, actual)
		}
	}
}

func TestConfig_RecordCommand(t *testing.T) {
	var tests = []struct {
		cfg      util.ConfigFile
		env      map[string]string
		expected string
	}{
		{
			util.ConfigFile{},
			map[string]string{},
			"/bin/sh",
		},
		{
			util.ConfigFile{},
			map[string]string{"SHELL": "/bin/bash"},
			"/bin/bash",
		},
		{
			util.ConfigFile{Record: util.ConfigRecord{Command: "foo -l"}},
			map[string]string{"SHELL": "/bin/bash"},
			"foo -l",
		},
	}

	for _, test := range tests {
		cfg := util.Config{&test.cfg, test.env}
		actual := cfg.RecordCommand()

		if actual != test.expected {
			t.Errorf(`expected "%v", got "%v"`, test.expected, actual)
		}
	}
}

func TestConfig_RecordMaxWait(t *testing.T) {
	var tests = []struct {
		cfg      util.ConfigFile
		expected uint
	}{
		{
			util.ConfigFile{},
			0,
		},
		{
			util.ConfigFile{Record: util.ConfigRecord{MaxWait: 1}},
			1,
		},
	}

	for _, test := range tests {
		cfg := util.Config{&test.cfg, nil}
		actual := cfg.RecordMaxWait()

		if actual != test.expected {
			t.Errorf(`expected "%v", got "%v"`, test.expected, actual)
		}
	}
}

func TestConfig_RecordYes(t *testing.T) {
	var tests = []struct {
		cfg      util.ConfigFile
		expected bool
	}{
		{
			util.ConfigFile{},
			false,
		},
		{
			util.ConfigFile{Record: util.ConfigRecord{Yes: true}},
			true,
		},
	}

	for _, test := range tests {
		cfg := util.Config{&test.cfg, nil}
		actual := cfg.RecordYes()

		if actual != test.expected {
			t.Errorf(`expected "%v", got "%v"`, test.expected, actual)
		}
	}
}

func TestConfig_PlaybackMaxWait(t *testing.T) {
	var tests = []struct {
		cfg      util.ConfigFile
		expected uint
	}{
		{
			util.ConfigFile{},
			0,
		},
		{
			util.ConfigFile{Playback: util.ConfigPlayback{MaxWait: 1}},
			1,
		},
	}

	for _, test := range tests {
		cfg := util.Config{&test.cfg, nil}
		actual := cfg.PlaybackMaxWait()

		if actual != test.expected {
			t.Errorf(`expected "%v", got "%v"`, test.expected, actual)
		}
	}
}
