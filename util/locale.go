package util

import (
	"os"
	"strings"
)

var usAscii = "US-ASCII"

func GetLocaleCharset() string {
	locale := FirstNonBlank(os.Getenv("LC_ALL"), os.Getenv("LC_CTYPE"), os.Getenv("LANG"))
	parts := strings.Split(locale, ".")

	if len(parts) == 2 {
		return parts[1]
	}

	return usAscii
}

func IsUtf8Locale() bool {
	charset := GetLocaleCharset()
	return charset == "utf-8" || charset == "UTF-8"
}
