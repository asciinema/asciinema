NAME=asciinema
VERSION=`python3 -c "import asciinema; print(asciinema.__version__)"`

test: test-unit test-integration

test-unit:
	nosetests

test-integration:
	tests/integration.sh

release: test tag push

release-test: test push-test

tag:
	git tag | grep "v$(VERSION)" && echo "Tag v$(VERSION) exists" && exit 1 || true
	git tag -s -m "Releasing $(VERSION)" v$(VERSION)
	git push --tags

push:
	python3 setup.py sdist upload -r pypi

push-test:
	python3 setup.py sdist upload -r pypitest

release: test tag push

.PHONY: test test-unit test-integration release release-test tag push push-test
