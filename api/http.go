package api

import (
	"bytes"
	"io"
	"mime/multipart"
	"net/http"
	"strings"
)

type HTTP interface {
	PostForm(string, string, string, map[string]string, map[string]io.Reader) (*http.Response, error)
}

type HTTPClient struct{}

func (c *HTTPClient) PostForm(url, username, password string, headers map[string]string, files map[string]io.Reader) (*http.Response, error) {
	req, err := createPostRequest(url, username, password, headers, files)
	if err != nil {
		return nil, err
	}

	client := &http.Client{}

	return client.Do(req)
}

func createPostRequest(url, username, password string, headers map[string]string, files map[string]io.Reader) (*http.Request, error) {
	body, contentType, err := multiPartBody(url, files)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequest("POST", url, body)
	if err != nil {
		return nil, err
	}

	setHeaders(req, contentType, username, password, headers)

	return req, nil
}

func setHeaders(req *http.Request, contentType, username, password string, headers map[string]string) {
	req.SetBasicAuth(username, password)

	req.Header.Set("Content-Type", contentType)

	for name, value := range headers {
		req.Header.Set(name, value)
	}
}

func multiPartBody(url string, files map[string]io.Reader) (io.Reader, string, error) {
	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)

	if files != nil {
		for name, reader := range files {
			err := addFormFile(writer, name, reader)
			if err != nil {
				return nil, "", err
			}
		}
	}

	err := writer.Close()
	if err != nil {
		return nil, "", err
	}

	return body, writer.FormDataContentType(), nil
}

func addFormFile(writer *multipart.Writer, name string, reader io.Reader) error {
	items := strings.Split(name, ":")
	fieldname := items[0]
	filename := items[1]

	part, err := writer.CreateFormFile(fieldname, filename)
	if err != nil {
		return err
	}

	_, err = io.Copy(part, reader)
	if err != nil {
		return err
	}

	return nil
}
