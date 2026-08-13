# Contributing to asciinema

Thank you for your interest in contributing! This document describes how
contributions work across the asciinema ecosystem:

- [asciinema](https://github.com/asciinema/asciinema) - the CLI: terminal
  session recorder, streamer and player
- [asciinema-player](https://github.com/asciinema/asciinema-player) - the web
  player
- [asciinema-server](https://github.com/asciinema/asciinema-server) - the
  hosting and streaming platform
- [agg](https://github.com/asciinema/agg) - the GIF generator
- [avt](https://github.com/asciinema/avt) - the virtual terminal library
- [docs](https://github.com/asciinema/asciinema.github.io) - the documentation
  site

## How asciinema is developed

asciinema is a passion project, built and maintained in spare time by a very
small team. The project follows a simple philosophy: a focused set of stable,
robust tools that work well together and stay out of your way.

Development is idea-driven rather than roadmap-driven. There is no formal
roadmap. New functionality usually gets added once an idea has proven itself,
often by resurfacing repeatedly in discussions and finding a shape that fits
the rest of the project. This deliberate pace is a feature: it's what keeps
asciinema small, dependable and easy to maintain for the years ahead.

## Where things go

- **Bug reports** belong in the issue tracker of the relevant repository.
  Our issue trackers are for bug reports only.
- **Feature ideas and change proposals** belong in the GitHub discussions
  ["Ideas" category](https://github.com/orgs/asciinema/discussions/categories/ideas)
  or on the [forum](https://discourse.asciinema.org/).
- **Questions and help requests** belong on the
  [forum](https://discourse.asciinema.org/) or in the GitHub discussions
  ["Q&A" category](https://github.com/orgs/asciinema/discussions/categories/q-a).
  The issue tracker is not a support channel.
- **Security issues** should be reported privately to admin@asciinema.org.
  Please do not describe vulnerabilities in public issues or discussions.

## Reporting bugs

Search existing issues first, including closed ones - your bug may already be
fixed or reported. If it's new, open an issue in the repository the bug
belongs to and fill in the issue template completely. Reliable reproduction
steps and environment details (OS, terminal, browser, versions) make the
difference between a quick fix and a stalled report.

If you're not sure whether something is a bug, or you can't reproduce it
reliably, start with a discussion or forum thread instead.

## Proposing features and changes

For anything beyond a small bug fix, please talk to us before writing code:

1. Start a thread in the
   ["Ideas" discussions category](https://github.com/orgs/asciinema/discussions/categories/ideas)
   or on the [forum](https://discourse.asciinema.org/).
2. Describe the problem you're solving, your proposed approach, and who would
   benefit from it.
3. If a maintainer gives the idea a green light, a pull request is very
   welcome. Link the thread in your PR. A green light means the idea is worth
   exploring, not a commitment to merge a particular implementation.

Here's why we ask for this. When a change is merged, the responsibility for it
shifts to the maintainers: from that point on we adapt it during refactorings,
fix bugs in it, and support the people using it, for as long as the code
lives. Every merge is a long-term commitment, so we are deliberate about which
commitments we take on. Proposals that benefit a broad range of users and fit
the project's focus have the best chance of being accepted.

Pull requests opened without prior discussion may stay unreviewed for a long
time or be closed. Even useful, well-implemented changes may be declined when
they don't fit the project's direction or when we can't take on their
long-term maintenance.

If your change is specific to your workflow, or it's something we can't adopt
right now, maintaining it in your own fork is a perfectly good outcome, and
one we genuinely encourage. The licenses give you all the freedom you need,
and we're happy when asciinema code is useful even outside the main line.

## Pull requests

Bug fix PRs can be opened directly; reference the issue if there is one.
Feature PRs should follow a green-lit discussion (see above).

What we look for in a PR:

- Keep it focused and as small as practical. We may ask for a large change to
  be split into smaller ones.
- Match the style and conventions of the surrounding code, and run the
  repository's formatter before submitting.
- Make sure the test suite passes, and add tests covering new behavior.
- Understand every change you submit. You should be able to explain what it
  does, how it interacts with the rest of the project, and answer review
  questions yourself.
- Write the description yourself, in your own words: what the change does, why,
  and how you tested it. The same goes for discussion threads and review
  replies.

We care about the long-term shape of the codebase, so reviews can be picky
about naming, structure and consistency, even when a change already works.
Sometimes we'll ask for adjustments or suggest a different implementation
before merging. Review timelines vary with maintainer availability; your
patience is appreciated.

### In this repository

- Pull requests should target the `develop` branch. The `python` branch holds
  the legacy 2.x codebase and is not under active development.
- Verify your changes with `cargo test`.

## AI-assisted contributions

If you use AI coding tools, note that in your PR description: which tool, and
to what extent (e.g. "wrote the first draft, which I then reviewed and
reworked").

The expectations above apply no matter how a change was produced. Reviewing a
change is often more work than writing it, and you are the one who needs to
understand, test and stand behind what you submit.

## Community

- [Forum](https://discourse.asciinema.org/)
- [GitHub discussions](https://github.com/orgs/asciinema/discussions)
- [Matrix](https://matrix.to/#/#asciinema:matrix.org)
- [IRC](https://web.libera.chat/#asciinema) (#asciinema on Libera.Chat)
