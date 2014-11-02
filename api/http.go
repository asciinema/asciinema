package api

import (
	"bytes"
	"io"
	"mime/multipart"
	"net/http"
	"strings"
)

type HTTP interface {
	PostForm(string, string, string, map[string]io.Reader) (*http.Response, error)
}

type HttpClient struct{}

func (c *HttpClient) PostForm(url, username, password string, files map[string]io.Reader) (*http.Response, error) {
	body, contentType, err := c.multiPartBody(url, files)
	if err != nil {
		return nil, err
	}

	client := &http.Client{}

	req, err := http.NewRequest("POST", url, body)
	if err != nil {
		return nil, err
	}

	req.Header.Set("Content-Type", contentType)
	req.SetBasicAuth(username, password)

	return client.Do(req)
}

func (c *HttpClient) multiPartBody(url string, files map[string]io.Reader) (io.Reader, string, error) {
	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)

	if files != nil {
		for name, reader := range files {
			items := strings.Split(name, ":")
			fieldname := items[0]
			filename := items[1]

			part, err := writer.CreateFormFile(fieldname, filename)
			if err != nil {
				return nil, "", err
			}

			_, err = io.Copy(part, reader)
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
