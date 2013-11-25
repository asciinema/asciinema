# Asciinema

[![Build Status](https://travis-ci.org/sickill/asciinema.png?branch=master)](https://travis-ci.org/sickill/asciinema)

Command line client for [asciinema.org](http://asciinema.org) service.

## Installation

The latest __stable version__ of asciinema can always be installed or updated
to via [pip](http://www.pip-installer.org/en/latest/index.html) (prefered) or
easy\_install:

    sudo pip install --upgrade asciinema

Alternatively:

    sudo easy_install asciinema

Or, you can install the __development version__ directly from GitHub:

    sudo pip install --upgrade https://github.com/sickill/asciinema/tarball/master

Arch Linux users can install the
[AUR](https://aur.archlinux.org/packages/asciinema/) package:

    yaourt -S asciinema

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

## Authors

Developed with passion by [Marcin Kulik](http://ku1ik.com) and great open
source [contributors](https://github.com/sickill/asciinema/contributors)

## Copyright

Copyright &copy; 2011-2013 Marcin Kulik. See LICENSE.txt for details.
