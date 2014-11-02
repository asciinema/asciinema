package api

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"time"
)

type Frame struct {
	Delay float64
	Data  []byte
}

type Api interface {
	CreateAsciicast([]Frame, time.Duration, int, int, string, string) (string, error)
}

func New(url, token string) *AsciinemaApi {
	return &AsciinemaApi{
		url:   url,
		token: token,
		http:  &HttpClient{},
	}
}

type AsciinemaApi struct {
	url   string
	token string
	http  HTTP
}

func (a *AsciinemaApi) CreateAsciicast(frames []Frame, duration time.Duration, cols, rows int, command, title string) (string, error) {
	files := map[string]io.Reader{
		"asciicast[stdout]:stdout":             gzippedDataReader(frames),
		"asciicast[stdout_timing]:stdout.time": gzippedTimingReader(frames),
		"asciicast[meta]:meta.json":            metadataReader(duration, cols, rows, command, title),
	}
	// TODO: set proper user agent

	response, err := a.http.PostForm(a.url+"/api/asciicasts", os.Getenv("USER"), a.token, files)
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
	metadata := map[string]interface{}{
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

	buf := &bytes.Buffer{}
	encoder := json.NewEncoder(buf)
	encoder.Encode(metadata)

	return buf
}
