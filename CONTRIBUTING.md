# Contributing to asciinema

First, if you're opening a GitHub issue make sure it goes to the correct
repository:

- [asciinema/asciinema](https://github.com/asciinema/asciinema/issues) - command-line recorder
- [asciinema/asciinema-server](https://github.com/asciinema/asciinema-server/issues) - public website hosting recordings
- [asciinema/asciinema-player](https://github.com/asciinema/asciinema-player/issues) - player

## Reporting bugs

Open an issue in GitHub issue tracker.

Tell us what's the problem and include steps to reproduce it (reliably).
Including your OS/browser/terminal name and version in the report would be
great.

## Submitting patches with bug fixes

If you found a bug and made a patch for it:

1. Make sure your changes pass the [pre-commit](https://pre-commit.com/)
   [hooks](.pre-commit-config.yaml). You can install the hooks in your work
   tree by running `pre-commit install` in your checked out copy.
1. Make sure all tests pass. If you add new functionality, add new tests.
1. Send us a pull request, including a description of the fix (referencing an
   existing issue if there's one).

## Requesting new features

We welcome all ideas.

If you believe most asciinema users would benefit from implementing your idea
then feel free to open a GitHub issue. However, as this is an open-source
project maintained by a small team of volunteers we simply can't implement all
of them due to limited resources. Please keep that in mind.

## Proposing features/changes (pull requests)

If you want to propose code change, either introducing a new feature or
improving an existing one, please first discuss this with asciinema team. You
can simply open a separate issue for a discussion or join #asciinema IRC
channel on Libera.Chat.

## Reporting security issues

If you found a security issue in asciinema please contact us at
admin@asciinema.org. For the benefit of all asciinema users please **do
not** publish details of the vulnerability in a GitHub issue.

The PGP key below (1eb33a8760dec34b) can be used when sending encrypted email
to or verifying responses from admin@asciinema.org.

```Public Key
-----BEGIN PGP PUBLIC KEY BLOCK-----
Version: GnuPG v2

mQENBFRH/yQBCADwC8fadhrTTqCFEcQ8ex82FE24b2frRC3fvkFeKsY+v2lniYmZ
wJ+qsd3cEv5uctCl+lQjrqhJrBx5DnZpCMw85vNuOhz/wjzn7efTISUF+HlnhiZd
tN3FPbk4uu+1JiiZ7SEvH+I4JjM46Vx6wPZ9en79u8VPMLJ24F81Rar62oiMuL29
PGV7CdG+ErUHEQfN1qLaZNQqkPCQSAouxooNqXKjs/mmz2651FrP8TKVr2f6B/2O
YJ++H9SoIp7Ly+/fEjgmdaZnGqfxnBC+Pm82tZguprWeh8pdiu9ieJswr4S9tRms
h2+eht8PWwkaOOhcFdZLnJFoXHOPzHilQVutABEBAAG0KUFzY2lpbmVtYSBTdXBw
b3J0IDxzdXBwb3J0QGFzY2lpbmVtYS5vcmc+iQE4BBMBAgAiBQJUR/8kAhsDBgsJ
CAcDAgYVCAIJCgsEFgIDAQIeAQIXgAAKCRAeszqHYN7DSyCeCADS9Jk7Ibl2f+2K
eZ4XmYU0UxU55EtHZBd34yF+FGbl4doQhnKcRqT5lKLfYk4x3LzzPAHNSbRS05/K
fw8l72GLHY01U/3slAixphIR8LwVyqPxwelTqLzkDvcK1TTTFnOM/XUT1ymNUS7i
6Bs889I4I8bPrnt1XK+W35/SqZbBAWotdidCbI/oKQgffCbVsH/Im5pnXTapvf/l
sRUpB2fp7vD5+ycKDcB5CqbtnsPU9vCPL11GG3ijwQBgnPc0fKanUHb3IMElQ0ju
8IYTZjpPe7bIV3V3nYZvdO41IYLCHhRpvNt4BO2amQoGyqTqGHr/rCY1aEToDG2c
cOdsEOmuuQENBFRH/yQBCACsR59NPSwGoK4zGgzDjuY7yLab2Tq1Jg1c038lA23G
t3H9aOpVbeYGvDPYLHi2y1cCNv19nzs5/k/LAflhTcgPjipTHQ2ojDG+MNfO4qyH
3JFhm1WUw6zxFjBXfsZhoCKTNHZkzH+d0jeutbBq/Rd77sLjN/VVTLfzJCZhyhKD
VEyO6DYaANZn1B/xx84WdxqqiQsLELOCQVUCG7HzbQAmx7lYYIUAwUoFTrBeBd+d
sN7htw3j7le99EiccqMXceZd2W9cAlRfXcjHtvbtkbJTcsvANSUSU10q5uuT3f6l
NftTLWOGZnu/rFU/ow5ipKft0ygfJKpMHD+AoLkiRIajABEBAAGJAR8EGAECAAkF
AlRH/yQCGwwACgkQHrM6h2Dew0tG1wgAqOkkSznwF+6muK88GgrgasqnIq2t2VkN
fTEKmykgSuMxiN4bsNLc4FQECZqIcL7zGuD6fFnsnO6Hg36R4rYGFSEsjjN7rXj0
QLnrJJLZV0oA6Q77fUqdB0he7uJm+nlQjUv8HNJwp1oIyhhHz/r1kTHUlX+bEMO3
Khc96UnE7nzwPBCbUvKuHJQY6K2ms1wgr9ELXjF1KVU9QtBtG2/XWRGDHDwQKxnW
+2pRVtn2xNJ9rBipGG86ZU88vurYjgPZrXaex3M1QGD/8+9Wlp/TR7YUzjiZbtwc
6mpG4SUlwZheX9RbTRdjnLr7Qy+CddOWvGxebgk23/U90KrDyHDHig==
=2M/2
-----END PGP PUBLIC KEY BLOCK-----
```
