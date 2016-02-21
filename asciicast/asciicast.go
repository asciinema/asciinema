package asciicast

import (
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"os"
	"strings"

	"github.com/asciinema/asciinema/Godeps/_workspace/src/golang.org/x/net/html"
)

type Env struct {
	Term  string `json:"TERM"`
	Shell string `json:"SHELL"`
}

type Duration float64

func (d Duration) MarshalJSON() ([]byte, error) {
	return []byte(fmt.Sprintf(`%.6f`, d)), nil
}

type Asciicast struct {
	Version  int      `json:"version"`
	Width    int      `json:"width"`
	Height   int      `json:"height"`
	Duration Duration `json:"duration"`
	Command  string   `json:"command"`
	Title    string   `json:"title"`
	Env      *Env     `json:"env"`
	Stdout   []Frame  `json:"stdout"`
}

func NewAsciicast(width, height int, duration float64, command, title string, frames []Frame, env map[string]string) *Asciicast {
	return &Asciicast{
		Version:  1,
		Width:    width,
		Height:   height,
		Duration: Duration(duration),
		Command:  command,
		Title:    title,
		Env:      &Env{Term: env["TERM"], Shell: env["SHELL"]},
		Stdout:   frames,
	}
}

func Save(asciicast *Asciicast, path string) error {
	bytes, err := json.MarshalIndent(asciicast, "", "  ")
	if err != nil {
		return err
	}

	err = ioutil.WriteFile(path, bytes, 0644)
	if err != nil {
		return err
	}

	return nil
}

// asciinema play file.json
// asciinema play https://asciinema.org/a/123.json
// asciinema play https://asciinema.org/a/123
// asciinema play ipfs://ipfs/QmbdpNCwqeZgnmAWBCQcs8u6Ts6P2ku97tfKAycE1XY88p
// asciinema play -

func getAttr(t *html.Token, name string) string {
	for _, a := range t.Attr {
		if a.Key == name {
			return a.Val
		}
	}

	return ""
}

func extractJSONURL(htmlDoc io.Reader) (string, error) {
	z := html.NewTokenizer(htmlDoc)

	for {
		tt := z.Next()

		switch {
		case tt == html.ErrorToken:
			return "", fmt.Errorf("expected alternate <link> not found in fetched HTML document")
		case tt == html.StartTagToken:
			t := z.Token()

			if t.Data == "link" && getAttr(&t, "rel") == "alternate" && getAttr(&t, "type") == "application/asciicast+json" {
				return getAttr(&t, "href"), nil
			}
		}
	}
}

func getSource(url string) (io.ReadCloser, error) {
	var source io.ReadCloser
	var isHTML bool
	var err error

	if strings.HasPrefix(url, "ipfs:/") {
		url = fmt.Sprintf("https://ipfs.io/%v", url[6:])
	} else if strings.HasPrefix(url, "fs:/") {
		url = fmt.Sprintf("https://ipfs.io/%v", url[4:])
	}

	if url == "-" {
		source = os.Stdin
	} else if strings.HasPrefix(url, "http://") || strings.HasPrefix(url, "https://") {
		resp, err := http.Get(url)

		if err != nil {
			return nil, err
		}

		if resp.StatusCode != 200 {
			resp.Body.Close()
			return nil, fmt.Errorf("got status %v when requesting %v", resp.StatusCode, url)
		}

		source = resp.Body

		if strings.HasPrefix(resp.Header.Get("Content-Type"), "text/html") {
			isHTML = true
		}
	} else {
		source, err = os.Open(url)
		if err != nil {
			return nil, err
		}

		if strings.HasSuffix(url, ".html") {
			isHTML = true
		}
	}

	if isHTML {
		defer source.Close()
		url, err = extractJSONURL(source)
		if err != nil {
			return nil, err
		}

		return getSource(url)
	}

	return source, nil
}

func Load(url string) (*Asciicast, error) {
	source, err := getSource(url)
	if err != nil {
		return nil, err
	}
	defer source.Close()

	dec := json.NewDecoder(source)
	asciicast := &Asciicast{}

	if err = dec.Decode(asciicast); err != nil {
		return nil, err
	}

	return asciicast, nil
}
