package util

import "strings"

var usAscii = "US-ASCII"

func GetLocaleCharset(env map[string]string) string {
	locale := FirstNonBlank(env["LC_ALL"], env["LC_CTYPE"], env["LANG"])
	parts := strings.Split(locale, ".")

	if len(parts) == 2 {
		return parts[1]
	}

	return usAscii
}

func IsUtf8Locale(env map[string]string) bool {
	charset := GetLocaleCharset(env)
	return charset == "utf-8" || charset == "UTF-8"
}
