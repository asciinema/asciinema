package api

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
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

type Api interface {
	CreateAsciicast([]Frame, time.Duration, int, int, string, string) (string, error)
}

type AsciinemaApi struct {
	url     string
	token   string
	version string
	http    HTTP
}

func New(url, token, version string) *AsciinemaApi {
	return &AsciinemaApi{
		url:     url,
		token:   token,
		version: version,
		http:    &HttpClient{},
	}
}

func (a *AsciinemaApi) CreateAsciicast(frames []Frame, duration time.Duration, cols, rows int, command, title string) (string, error) {
	response, err := a.http.PostForm(
		a.createUrl(),
		a.user(),
		a.token,
		a.createHeaders(),
		a.createFiles(frames, duration, cols, rows, command, title),
	)

	if err != nil {
		return "", err
	}
	defer response.Body.Close()

	body := &bytes.Buffer{}
	_, err = body.ReadFrom(response.Body)
	if err != nil {
		return "", err
	}

	// TODO: handle non-200 statuses

	return body.String(), nil
}

func (a *AsciinemaApi) createUrl() string {
	return a.url + "/api/asciicasts"
}

func (a *AsciinemaApi) user() string {
	return os.Getenv("USER")
}

func (a *AsciinemaApi) createHeaders() map[string]string {
	return map[string]string{
		"User-Agent": fmt.Sprintf("asciinema/%s %s/%s %s-%s", a.version, runtime.Compiler, runtime.Version(), runtime.GOOS, runtime.GOARCH),
	}
}

func (a *AsciinemaApi) createFiles(frames []Frame, duration time.Duration, cols, rows int, command, title string) map[string]io.Reader {
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
