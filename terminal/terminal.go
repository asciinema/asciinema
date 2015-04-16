package terminal

import (
	"io"
	"os"
	"os/exec"
	"os/signal"
	"syscall"
	"time"

	"github.com/asciinema/asciinema/Godeps/_workspace/src/code.google.com/p/go.crypto/ssh/terminal"
	"github.com/asciinema/asciinema/Godeps/_workspace/src/github.com/creack/termios/raw"
	"github.com/asciinema/asciinema/Godeps/_workspace/src/github.com/kr/pty"
	"github.com/asciinema/asciinema/ptyx"
	"github.com/asciinema/asciinema/util"
)

type Terminal interface {
	Size() (int, int, error)
	Record(string, io.Writer) error
	Write([]byte) error
}

type Pty struct {
	Stdin  *os.File
	Stdout *os.File
}

func NewTerminal() Terminal {
	return &Pty{Stdin: os.Stdin, Stdout: os.Stdout}
}

func (p *Pty) Size() (int, int, error) {
	return pty.Getsize(p.Stdout)
}

func (p *Pty) Record(command string, stdoutCopy io.Writer) error {
	// start command in pty
	cmd := exec.Command("sh", "-c", command)
	cmd.Env = append(os.Environ(), "ASCIINEMA_REC=1")
	master, err := pty.Start(cmd)
	if err != nil {
		return err
	}
	defer master.Close()

	// install WINCH signal handler
	signals := make(chan os.Signal, 1)
	signal.Notify(signals, syscall.SIGWINCH)
	defer signal.Stop(signals)
	go func() {
		for _ = range signals {
			p.resize(master)
		}
	}()
	defer close(signals)

	// put stdin in raw mode (if it's a tty)
	fd := p.Stdin.Fd()
	if terminal.IsTerminal(int(fd)) {
		oldState, err := raw.MakeRaw(fd)
		if err != nil {
			return err
		}
		defer raw.TcSetAttr(fd, oldState)
	}

	// do initial resize
	p.resize(master)

	// start stdin -> master copying
	stop := util.Copy(master, p.Stdin)

	// copy pty master -> p.stdout & stdoutCopy
	stdout := io.MultiWriter(p.Stdout, stdoutCopy)
	stdoutWaitChan := make(chan struct{})
	go func() {
		io.Copy(stdout, master)
		stdoutWaitChan <- struct{}{}
	}()

	// wait for the process to exit and reap it
	cmd.Wait()

	// wait for master -> stdout copying to finish
	//
	// sometimes after process exits reading from master blocks forever (race condition?)
	// we're using timeout here to overcome this problem
	select {
	case <-stdoutWaitChan:
	case <-time.After(200 * time.Millisecond):
	}

	// stop stdin -> master copying
	stop()

	return nil
}

func (p *Pty) Write(data []byte) error {
	_, err := p.Stdout.Write(data)
	if err != nil {
		return err
	}

	err = p.Stdout.Sync()
	if err != nil {
		return err
	}

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
