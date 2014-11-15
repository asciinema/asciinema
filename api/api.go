package api

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"runtime"
	"time"
)

type Frame struct {
	Delay float64
	Data  []byte
}

type API interface {
	CreateAsciicast([]Frame, time.Duration, int, int, string, string) (string, error)
}

type AsciinemaAPI struct {
	url     string
	token   string
	version string
	http    HTTP
}

func New(url, token, version string) *AsciinemaAPI {
	return &AsciinemaAPI{
		url:     url,
		token:   token,
		version: version,
		http:    &HTTPClient{},
	}
}

func (a *AsciinemaAPI) CreateAsciicast(frames []Frame, duration time.Duration, cols, rows int, command, title string) (string, error) {
	response, err := a.http.PostForm(
		a.createURL(),
		a.username(),
		a.token,
		a.createHeaders(),
		a.createFiles(frames, duration, cols, rows, command, title),
	)

	if err != nil {
		return "", fmt.Errorf("Connection failed (%v)", err.Error())
	}
	defer response.Body.Close()

	if response.StatusCode != 200 && response.StatusCode != 201 {
		if response.StatusCode == 404 {
			return "", errors.New("Your client version is no longer supported. Please upgrade to the latest version.")
		}
		if response.StatusCode == 503 {
			return "", errors.New("The server is down for maintenance. Try again in a minute.")
		}

		return "", errors.New("HTTP status: " + response.Status)
	}

	body := &bytes.Buffer{}
	_, err = body.ReadFrom(response.Body)
	if err != nil {
		return "", err
	}

	return body.String(), nil
}

func (a *AsciinemaAPI) createURL() string {
	return a.url + "/api/asciicasts"
}

func (a *AsciinemaAPI) username() string {
	return os.Getenv("USER")
}

func (a *AsciinemaAPI) createHeaders() map[string]string {
	return map[string]string{
		"User-Agent": fmt.Sprintf("asciinema/%s %s/%s %s-%s", a.version, runtime.Compiler, runtime.Version(), runtime.GOOS, runtime.GOARCH),
	}
}

func (a *AsciinemaAPI) createFiles(frames []Frame, duration time.Duration, cols, rows int, command, title string) map[string]io.Reader {
	return map[string]io.Reader{
		"asciicast[stdout]:stdout":             gzippedDataReader(frames),
		"asciicast[stdout_timing]:stdout.time": gzippedTimingReader(frames),
		"asciicast[meta]:meta.json":            metadataReader(duration, cols, rows, command, title),
	}
}

func gzippedDataReader(frames []Frame) io.Reader {
	data := &bytes.Buffer{}
	w := gzip.NewWriter(data)

	for _, frame := range frames {
		w.Write(frame.Data)
	}

	w.Close()

	return data
}

func gzippedTimingReader(frames []Frame) io.Reader {
	timing := &bytes.Buffer{}
	w := gzip.NewWriter(timing)

	for _, frame := range frames {
		w.Write([]byte(fmt.Sprintf("%f %d\n", frame.Delay, len(frame.Data))))
	}

	w.Close()

	return timing
}

func metadataReader(duration time.Duration, cols, rows int, command, title string) io.Reader {
	buf := &bytes.Buffer{}
	encoder := json.NewEncoder(buf)
	encoder.Encode(metadata(duration, cols, rows, command, title))

	return buf
}

func metadata(duration time.Duration, cols, rows int, command, title string) map[string]interface{} {
	return map[string]interface{}{
		"duration": duration.Seconds(),
		"title":    title,
		"command":  command,
		"shell":    os.Getenv("SHELL"),
		"term": map[string]interface{}{
			"type":    os.Getenv("TERM"),
			"columns": cols,
			"lines":   rows,
		},
	}
}
