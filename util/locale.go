package util

import (
	"os"
	"strings"
)

var usAscii = "US-ASCII"

func GetLocaleCharset() string {
	for _, key := range []string{"LC_ALL", "LC_CTYPE", "LANG"} {
		value := os.Getenv(key)
		if value != "" {
			parts := strings.Split(value, ".")

			if len(parts) == 2 {
				return parts[1]
			} else {
				return usAscii
			}
		}
	}

	return usAscii
}

func IsUtf8Locale() bool {
	charset := GetLocaleCharset()
	return charset == "utf-8" || charset == "UTF-8"
}
