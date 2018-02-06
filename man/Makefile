VERSION=`python3 -c "import asciinema; print(asciinema.__version__)"`

asciinema.1: asciinema.1.md
	pandoc asciinema.1.md -s -t man -o asciinema.1 -V header:"Version $(VERSION), `date +%Y-%m-%d`"

