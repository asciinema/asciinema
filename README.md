# Asciinema

[![PyPI version](https://badge.fury.io/py/asciinema.png)](http://badge.fury.io/py/asciinema)
[![Build Status](https://travis-ci.org/sickill/asciinema.png?branch=master)](https://travis-ci.org/sickill/asciinema)
[![Downloads](https://pypip.in/d/asciinema/badge.png)](https://pypi.python.org/pypi/asciinema)

Command line client for [asciinema.org](https://asciinema.org) service.

## Installation

The latest __stable version__ of asciinema can always be installed or updated
to via [pip](http://www.pip-installer.org/en/latest/index.html) (prefered) or
easy\_install:

    sudo pip install --upgrade asciinema

Alternatively:

    sudo easy_install asciinema

Or, you can install the __development version__ directly from GitHub:

    sudo pip install --upgrade https://github.com/sickill/asciinema/tarball/master

See [installation docs](https://asciinema.org/docs/installation) for more
options (Ubuntu, Fedora, Arch, Gentoo etc).

## Usage

Record:

    $ asciinema rec
    d734ae3

Replay:

    $ asciinema play d734ae3

Edit:

    $ asciinema edit d734ae3

Publish:

    $ asciinema push d734ae3

## Contributing

If you want to contribute to this project check out
[Contributing](https://asciinema.org/contributing) page.

## Authors

Developed with passion by [Marcin Kulik](http://ku1ik.com) and great open
source [contributors](https://github.com/sickill/asciinema/contributors)

## Copyright

Copyright &copy; 2011-2013 Marcin Kulik. See LICENSE.txt for details.
