package util

import "strings"

var usAscii = "US-ASCII"

func extractCharset(locale, defaultCharset string) string {
	parts := strings.Split(locale, ".")

	if len(parts) == 2 {
		return parts[1]
	}

	return defaultCharset
}

func GetLocaleCharset(env map[string]string) string {
	if env["LC_ALL"] != "" {
		return extractCharset(env["LC_ALL"], usAscii)
	}

	if env["LC_CTYPE"] != "" {
		return extractCharset(env["LC_CTYPE"], env["LC_CTYPE"])
	}

	if env["LANG"] != "" {
		return extractCharset(env["LANG"], usAscii)
	}

	return usAscii
}

func IsUtf8Locale(env map[string]string) bool {
	charset := GetLocaleCharset(env)
	return charset == "utf-8" || charset == "UTF-8" || charset == "utf8"
}
