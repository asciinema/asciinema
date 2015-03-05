package main

import (
	"fmt"
	"os"
	"strconv"

	"github.com/asciinema/asciinema-cli/Godeps/_workspace/src/github.com/docopt/docopt-go"
	"github.com/asciinema/asciinema-cli/api"
	"github.com/asciinema/asciinema-cli/commands"
	"github.com/asciinema/asciinema-cli/util"
)

const Version = "1.0.0.rc1"

var GitCommit string // populated during build

var usage = `Record and share your terminal sessions, the right way.

Usage:
  asciinema rec [-c <command>] [-t <title>] [-w <sec>] [-y] [<filename>]
  asciinema play <filename>
  asciinema upload <filename>
  asciinema auth
  asciinema -h | --help
  asciinema --version

Commands:
  rec             Record terminal session
  play            Replay terminal session
  upload          Upload locally saved terminal session to asciinema.org
  auth            Assign local API token to asciinema.org account

Options:
  -c, --command=<command>  Specify command to record, defaults to $SHELL
  -t, --title=<title>      Specify title of the asciicast
  -w, --max-wait=<sec>     Reduce recorded terminal inactivity to max <sec> seconds
  -y, --yes                Answer yes to all prompts (e.g. upload confirmation)
  -h, --help               Show this message
  --version                Show version`

func cmdName(args map[string]interface{}) string {
	for _, cmd := range []string{"rec", "play", "upload", "auth"} {
		if args[cmd].(bool) {
			return cmd
		}
	}

	return ""
}

func stringArg(args map[string]interface{}, name string) string {
	val := args[name]

	if val != nil {
		return val.(string)
	} else {
		return ""
	}
}

func boolArg(args map[string]interface{}, name string) bool {
	return args[name].(bool)
}

func uintArg(args map[string]interface{}, name string) uint {
	val := args[name]

	if val != nil {
		number, err := strconv.ParseUint(val.(string), 10, 0)

		if err == nil {
			return uint(number)
		}
	}

	return 0
}

func firstNonBlank(candidates ...string) string {
	for _, candidate := range candidates {
		if candidate != "" {
			return candidate
		}
	}

	return ""
}

func formatVersion() string {
	var commitInfo string

	if GitCommit != "" {
		commitInfo = "-" + GitCommit
	}

	return fmt.Sprintf("asciinema %v%v\n", Version, commitInfo)
}

func main() {
	if !util.IsUtf8Locale() {
		fmt.Println("asciinema needs a UTF-8 native locale to run. Check the output of `locale` command.")
		os.Exit(1)
	}

	cfg, err := util.LoadConfig()
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}

	api := api.New(cfg.API.URL, cfg.API.Token, Version)
	args, _ := docopt.Parse(usage, nil, true, formatVersion(), false)

	switch cmdName(args) {
	case "rec":
		command := firstNonBlank(stringArg(args, "--command"), cfg.Record.Command, os.Getenv("SHELL"), "/bin/sh")
		title := stringArg(args, "--title")
		assumeYes := boolArg(args, "--yes")
		maxWait := uintArg(args, "--max-wait")
		filename := stringArg(args, "<filename>")
		cmd := commands.NewRecordCommand(api, cfg)
		err = cmd.Execute(command, title, assumeYes, maxWait, filename)

	case "play":
		filename := stringArg(args, "<filename>")
		cmd := commands.NewPlayCommand()
		err = cmd.Execute(filename)

	case "upload":
		filename := stringArg(args, "<filename>")
		cmd := commands.NewUploadCommand(api)
		err = cmd.Execute(filename)

	case "auth":
		cmd := commands.NewAuthCommand(cfg)
		err = cmd.Execute()
	}

	if err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(1)
	}
}
