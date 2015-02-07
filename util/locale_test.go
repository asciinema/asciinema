package util_test

import (
	"os"
	"testing"

	"github.com/asciinema/asciinema-cli/util"
)

func TestGetLocaleCharset(t *testing.T) {
	var tests = []struct {
		lcAll          string
		lcCtype        string
		lang           string
		expectedResult string
	}{
		{"pl_PL.UTF-8", "pl_PL.ISO-8859-1", "pl_PL.ISO-8859-2", "UTF-8"},
		{"", "pl_PL.ISO-8859-1", "pl_PL.ISO-8859-2", "ISO-8859-1"},
		{"", "", "pl_PL.ISO-8859-2", "ISO-8859-2"},
		{"", "", "", "US-ASCII"},
		{"UTF-8", "pl_PL.ISO-8859-1", "pl_PL.ISO-8859-2", "US-ASCII"},
		{"", "ISO-8859-1", "pl_PL.ISO-8859-2", "US-ASCII"},
		{"", "", "ISO-8859-2", "US-ASCII"},
	}

	for _, test := range tests {
		os.Setenv("LC_ALL", test.lcAll)
		os.Setenv("LC_CTYPE", test.lcCtype)
		os.Setenv("LANG", test.lang)

		if util.GetLocaleCharset() != test.expectedResult {
			t.Errorf("expected %v for %v", test.expectedResult, test)
		}
	}
}
