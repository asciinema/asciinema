NAME=asciinema
VERSION=0.9.1
AUTHOR=sickill
URL=https://github.com/$(AUTHOR)/$(NAME)

DIRS=bin
INSTALL_DIRS=`find $(DIRS) -type d 2>/dev/null`
INSTALL_FILES=`find $(DIRS) -type f 2>/dev/null`

PKG_DIR=pkg
PKG_NAME=$(NAME)-$(VERSION)
PKG=$(PKG_DIR)/$(PKG_NAME).tar.gz
SIG=$(PKG).asc

PREFIX?=/usr/local

pkg:
	mkdir $(PKG_DIR)

download: pkg
	wget -O $(PKG) $(URL)/archive/v$(VERSION).tar.gz

$(SIG): $(PKG)
	gpg --sign --detach-sign --armor $(PKG)
	git add $(PKG).asc
	git commit $(PKG).asc -m "Added PGP signature for v$(VERSION)"
	git push

verify: $(PKG) $(SIG)
	gpg --verify $(SIG) $(PKG)

clean:
	rm -f $(PKG) $(SIG)

all: $(PKG) $(SIG)

update-bin: bin/asciinema
	git add bin/asciinema
	git commit bin/asciinema -m "Update bin/asciinema for $(VERSION) release"
	git push

tag:
	git tag -s -m "Releasing $(VERSION)" v$(VERSION)
	git push --tags

sign: $(SIG)

release: update-bin tag download sign

install:
	for dir in $(INSTALL_DIRS); do mkdir -p $(PREFIX)/$$dir; done
	for file in $(INSTALL_FILES); do cp $$file $(PREFIX)/$$file; done

uninstall:
	for file in $(INSTALL_FILES); do rm -f $(PREFIX)/$$file; done

bin/asciinema: tmp/asciinema.zip
	echo '#!/usr/bin/env python2' > bin/asciinema
	cat tmp/asciinema.zip >> bin/asciinema
	chmod +x bin/asciinema

tmp/asciinema.zip: src/* src/commands/*
	mkdir -p tmp
	rm -f tmp/asciinema.zip
	cd src && zip -r - `find . -name \*.py` >../tmp/asciinema.zip

test: test-unit test-integration

test-unit:
	PYTHONPATH=tests nosetests `find tests -name "*_test.py"`

test-integration:
	tests/integration.sh

.PHONY: download sign verify clean test tag release install uninstall all
