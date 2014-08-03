package terminal

import (
	"io"
	"os"
	"os/exec"
	"os/signal"
	"syscall"

	"code.google.com/p/go.crypto/ssh/terminal"

	"github.com/asciinema/asciinema-cli/ptyx"
	"github.com/kr/pty"
)

type Terminal interface {
	Size() (int, int, error)
	Record(string, io.Writer) error
}

type Pty struct {
	Stdin  *os.File
	Stdout *os.File
}

func New() *Pty {
	return &Pty{Stdin: os.Stdin, Stdout: os.Stdout}
}

func (p *Pty) Size() (int, int, error) {
	return pty.Getsize(p.Stdout)
}

func (p *Pty) Record(command string, stdoutCopy io.Writer) error {
	// 1. start command in pty
	cmd := exec.Command("/bin/sh", "-c", command)
	cmd.Env = append(os.Environ(), "ASCIINEMA_REC=1")
	master, err := pty.Start(cmd)
	if err != nil {
		return err
	}
	defer master.Close()

	// 2. install WINCH signal handler
	signals := make(chan os.Signal, 1)
	signal.Notify(signals, syscall.SIGWINCH)
	defer signal.Stop(signals)
	go func() {
		for _ = range signals {
			p.resize(master)
		}
	}()
	defer close(signals)

	// 3. put stdin into raw mode (if it's a tty)
	fd := int(p.Stdin.Fd())
	if terminal.IsTerminal(fd) {
		oldState, err := terminal.MakeRaw(fd)
		if err != nil {
			return err
		}
		defer terminal.Restore(fd, oldState)
	}

	// 4. do initial resize
	p.resize(master)

	// 5. copy stdin -> pty master, pty master -> stdout
	go func() {
		io.Copy(master, p.Stdin)
	}()
	stdout := io.MultiWriter(p.Stdout, stdoutCopy)
	io.Copy(stdout, master)

	return nil
}

func (p *Pty) resize(f *os.File) {
	var rows, cols int

	if terminal.IsTerminal(int(p.Stdout.Fd())) {
		rows, cols, _ = p.Size()
	} else {
		rows = 24
		cols = 80
	}

	ptyx.Setsize(f, rows, cols)
}
