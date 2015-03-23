package util_test

import (
	"testing"

	"github.com/asciinema/asciinema/util"
)

func TestGetLocaleCharset(t *testing.T) {
	var tests = []struct {
		lcAll          string
		lcCtype        string
		lang           string
		expectedResult string
	}{
		{"pl_PL.UTF-8", "pl_PL.ISO-8859-1", "pl_PL.ISO-8859-2", "UTF-8"},
		{"cz_CS.utf8", "pl_PL.ISO-8859-1", "pl_PL.ISO-8859-2", "utf8"},
		{"", "pl_PL.ISO-8859-1", "pl_PL.ISO-8859-2", "ISO-8859-1"},
		{"", "", "pl_PL.ISO-8859-2", "ISO-8859-2"},
		{"", "", "", "US-ASCII"},
		{"UTF-8", "pl_PL.ISO-8859-1", "pl_PL.ISO-8859-2", "US-ASCII"},
		{"", "ISO-8859-1", "pl_PL.ISO-8859-2", "US-ASCII"},
		{"", "", "ISO-8859-2", "US-ASCII"},
	}

	for _, test := range tests {
		env := map[string]string{
			"LC_ALL":   test.lcAll,
			"LC_CTYPE": test.lcCtype,
			"LANG":     test.lang,
		}

		if util.GetLocaleCharset(env) != test.expectedResult {
			t.Errorf("expected %v for %v", test.expectedResult, test)
		}
	}
}
