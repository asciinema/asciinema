# Changelog

## 0.9.8 (2014-02-09)

* Rename user_token to api_token
* Improvements to test suite
* Send User-Agent including client version number, python version and platform
* Handle 503 status as server maintenance
* Handle 404 response as a request for client upgrade

## 0.9.7 (2013-10-07)

* Depend on requests==1.1.0, not 2.0

## 0.9.6 (2013-10-06)

* Remove install script
* Introduce proper python package: https://pypi.python.org/pypi/asciinema
* Make the code compatible with both python 2 and 3
* Use requests lib instead of urrlib(2)

## 0.9.5 (2013-10-04)

* Fixed measurement of total recording time
* Improvements to install script
* Introduction of Homebrew formula

## 0.9.4 (2013-10-03)

* Use python2.7 in shebang

## 0.9.3 (2013-10-03)

* Re-enable resetting of a terminal before and after recording
* Add Arch Linux source package

## 0.9.2 (2013-10-02)

* Use os.uname over running the uname command
* Add basic integration tests
* Make PtyRecorder test stable again
* Move install script out of bin dir

## 0.9.1 (2013-10-01)

* Split monolithic script into separate classes/files
* Remove upload queue
* Use python2 in generated binary's shebang
* Delay config file creation until user_token is requested
* Introduce command classes for handling cli commands
* Split the recorder into classes with well defined responsibilities
* Drop curl dependency, use urllib(2) for http requests

## 0.9.0 (2013-09-24)

* Project rename from "ascii.io" to "asciinema"

## ... limbo? ...

## 0.1 (2012-03-11)

* Initial release
