NAME    := asciinema
VERSION := $(shell python3 -c "import asciinema; print(asciinema.__version__)")

VIRTUAL_ENV ?= .venv

.PHONY: test
test: test.unit test.integration

.PHONY: test.unit
test.unit:
	pytest

.PHONY: test.integration
test.integration:
	tests/integration.sh

.PHONY: test.distros
test.distros:
	tests/distros.sh

.PHONY: release
release: test tag push

.PHONY: release.test
release.test: test push.test

.PHONY: .tag.exists
.tag.exists:
	@git tag \
		| grep -q "v$(VERSION)" \
		&& echo "Tag v$(VERSION) exists" \
		&& exit 1

.PHONY: tag
tag: .tag.exists
	git tag -s -m "Releasing $(VERSION)" v$(VERSION)
	git push origin v$(VERSION)

.PHONY: .venv
.venv:
	python3 -m venv $(VIRTUAL_ENV)

.PHONY: .pip
.pip: .venv
	. $(VIRTUAL_ENV)/bin/activate \
		&& python3 -m pip install --upgrade build twine

build: .pip
	. $(VIRTUAL_ENV)/bin/activate \
		&& python3 -m build .

install: build
	. $(VIRTUAL_ENV)/bin/activate \
		&& python3 -m pip install .

.PHONY: push
push: .pip build
	python3 -m twine upload dist/*

.PHONY: push.test
push.test: .pip build
	python3 -m twine upload --repository testpypi dist/*


.PHONY: clean
clean:
	rm -rf dist *.egg-info

clean.all: clean
	find .  \
		-type d \
		-name __pycache__ \
		-o -name .pytest_cache \
		-o -name .mypy_cache \
		-exec rm -r "{}" +
