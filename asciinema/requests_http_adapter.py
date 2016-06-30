import requests


class RequestsHttpAdapter:

    def post(self, url, fields={}, files={}, headers={}):
        response = requests.post(url, data=fields, files=files, headers=headers)

        status  = response.status_code
        headers = response.headers
        body    = response.text

        return (status, headers, body)
