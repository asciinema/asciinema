package util

import (
	"os"
	"regexp"
)

var charsetRegexp = regexp.MustCompile("(?i)\\.UTF-8$")

func IsUtf8Locale() bool {
	all := os.Getenv("LC_ALL")
	if charsetRegexp.FindStringSubmatch(all) != nil {
		return true
	}

	ctype := os.Getenv("LC_CTYPE")
	if charsetRegexp.FindStringSubmatch(ctype) != nil {
		return true
	}

	lang := os.Getenv("LANG")
	if charsetRegexp.FindStringSubmatch(lang) != nil {
		return true
	}

	return false
}
