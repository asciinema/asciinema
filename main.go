package main

import (
	"fmt"
	"os"
	"strconv"
	"strings"

	"github.com/asciinema/asciinema/Godeps/_workspace/src/github.com/docopt/docopt-go"
	"github.com/asciinema/asciinema/api"
	"github.com/asciinema/asciinema/commands"
	"github.com/asciinema/asciinema/util"
)

const Version = "1.1.1"

var usage = `Record and share your terminal sessions, the right way.

Usage:
  asciinema rec [-c <command>] [-t <title>] [-w <sec>] [-y] [<filename>]
  asciinema play [-w <sec>] <filename>
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

func uintArg(args map[string]interface{}, name string, defaultValue uint) uint {
	val := args[name]

	if val != nil {
		number, err := strconv.ParseUint(val.(string), 10, 0)

		if err == nil {
			return uint(number)
		}
	}

	return defaultValue
}

func formatVersion() string {
	return fmt.Sprintf("asciinema %v", Version)
}

func environment() map[string]string {
	env := map[string]string{}

	for _, keyval := range os.Environ() {
		pair := strings.SplitN(keyval, "=", 2)
		env[pair[0]] = pair[1]
	}

	return env
}

func main() {
	env := environment()

	if !util.IsUtf8Locale(env) {
		fmt.Println("asciinema needs a UTF-8 native locale to run. Check the output of `locale` command.")
		os.Exit(1)
	}

	cfg, err := util.GetConfig(env)
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}

	api := api.New(cfg.ApiUrl(), env["USER"], cfg.ApiToken(), Version)
	args, _ := docopt.Parse(usage, nil, true, formatVersion(), false)

	switch cmdName(args) {
	case "rec":
		command := util.FirstNonBlank(stringArg(args, "--command"), cfg.RecordCommand())
		title := stringArg(args, "--title")
		assumeYes := cfg.RecordYes() || boolArg(args, "--yes")
		maxWait := uintArg(args, "--max-wait", cfg.RecordMaxWait())
		filename := stringArg(args, "<filename>")
		cmd := commands.NewRecordCommand(api, env)
		err = cmd.Execute(command, title, assumeYes, maxWait, filename)

	case "play":
		maxWait := uintArg(args, "--max-wait", cfg.PlayMaxWait())
		filename := stringArg(args, "<filename>")
		cmd := commands.NewPlayCommand()
		err = cmd.Execute(filename, maxWait)

	case "upload":
		filename := stringArg(args, "<filename>")
		cmd := commands.NewUploadCommand(api)
		err = cmd.Execute(filename)

	case "auth":
		cmd := commands.NewAuthCommand(api)
		err = cmd.Execute()
	}

	if err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(1)
	}
}
