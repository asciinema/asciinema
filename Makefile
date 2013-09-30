all: build

build: bin/asciinema

bin/asciinema: tmp/asciinema.zip
	echo '#!/usr/bin/env python2' > bin/asciinema
	cat tmp/asciinema.zip >> bin/asciinema
	chmod +x bin/asciinema

tmp/asciinema.zip: src/__main__.py src/asciicast.py src/recorders.py src/timed_file.py src/uploader.py src/config.py src/options.py src/cli.py
	mkdir -p tmp
	rm -rf tmp/asciinema.zip
	cd src && zip ../tmp/asciinema.zip *.py
