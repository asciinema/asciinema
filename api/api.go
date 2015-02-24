package api

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"os"
	"runtime"
)

type API interface {
	UploadAsciicast(string) (string, error)
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

func (a *AsciinemaAPI) UploadAsciicast(path string) (string, error) {
	files, err := filesForUpload(path)
	if err != nil {
		return "", err
	}

	response, err := a.http.PostForm(
		a.urlForUpload(),
		a.username(),
		a.token,
		a.headersForUpload(),
		files,
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

func (a *AsciinemaAPI) urlForUpload() string {
	return a.url + "/api/asciicasts"
}

func (a *AsciinemaAPI) username() string {
	return os.Getenv("USER")
}

func (a *AsciinemaAPI) headersForUpload() map[string]string {
	return map[string]string{
		"User-Agent": fmt.Sprintf("asciinema/%s %s/%s %s-%s", a.version, runtime.Compiler, runtime.Version(), runtime.GOOS, runtime.GOARCH),
	}
}

func filesForUpload(path string) (map[string]io.ReadCloser, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}

	return map[string]io.ReadCloser{"asciicast:asciicast.json": file}, nil
}
