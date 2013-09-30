all: build

build: bin/asciinema

bin/asciinema: tmp/asciinema.zip
	echo '#!/usr/bin/env python2' > bin/asciinema
	cat tmp/asciinema.zip >> bin/asciinema
	chmod +x bin/asciinema

tmp/asciinema.zip: src/* src/commands/*
	mkdir -p tmp
	rm -f tmp/asciinema.zip
	cd src && zip -r ../tmp/asciinema.zip `find . -name \*.py`

test:
	PYTHONPATH=tests nosetests `find tests -name "*_test.py"`

.PHONY: test
