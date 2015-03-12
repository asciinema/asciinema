package api

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"runtime"
	"strings"
)

type API interface {
	AuthUrl() string
	UploadAsciicast(string) (string, string, error)
}

type AsciinemaAPI struct {
	url     string
	user    string
	token   string
	version string
	http    HTTP
}

func New(url, user, token, version string) *AsciinemaAPI {
	return &AsciinemaAPI{
		url:     url,
		user:    user,
		token:   token,
		version: version,
		http:    &HTTPClient{},
	}
}

func (a *AsciinemaAPI) AuthUrl() string {
	return fmt.Sprintf("%v/connect/%v", a.url, a.token)
}

func (a *AsciinemaAPI) UploadAsciicast(path string) (string, string, error) {
	files, err := filesForUpload(path)
	if err != nil {
		return "", "", err
	}

	response, err := a.makeUploadRequest(files)
	if err != nil {
		return "", "", fmt.Errorf("Connection failed (%v)", err.Error())
	}
	defer response.Body.Close()

	body, err := extractBody(response)
	if err != nil {
		return "", "", err
	}

	warn := extractWarningMessage(response)

	if response.StatusCode != 200 && response.StatusCode != 201 {
		return "", warn, handleError(response, body)
	}

	return body, warn, nil
}

func (a *AsciinemaAPI) makeUploadRequest(files map[string]io.ReadCloser) (*http.Response, error) {
	return a.http.PostForm(a.urlForUpload(), a.user, a.token, a.headersForUpload(), files)
}

func (a *AsciinemaAPI) urlForUpload() string {
	return a.url + "/api/asciicasts"
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

func extractWarningMessage(response *http.Response) string {
	parts := strings.SplitN(response.Header.Get("Warning"), " ", 2)

	if len(parts) == 2 {
		return parts[1]
	}

	return ""
}

func extractBody(response *http.Response) (string, error) {
	body := &bytes.Buffer{}

	_, err := body.ReadFrom(response.Body)
	if err != nil {
		return "", err
	}

	return body.String(), nil
}

func handleError(response *http.Response, body string) error {
	switch response.StatusCode {
	case 404:
		return errors.New("Your client version is no longer supported. Please upgrade to the latest version.")
	case 413:
		return errors.New("Sorry, your asciicast is too big.")
	case 422:
		return fmt.Errorf("Invalid asciicast: %v", body)
	case 504:
		return errors.New("The server is down for maintenance. Try again in a minute.")
	default:
		return errors.New("HTTP status: " + response.Status)
	}
}
