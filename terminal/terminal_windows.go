// +build windows

package terminal

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"os/signal"
	"sync"
	"syscall"
	"time"
	"unicode/utf16"
	"unsafe"

	"github.com/mattn/go-colorable"
	"github.com/mattn/go-isatty"
)

var (
	kernel32                       = syscall.NewLazyDLL("kernel32.dll")
	procGetConsoleScreenBufferInfo = kernel32.NewProc("GetConsoleScreenBufferInfo")
	procSetConsoleWindowInfo       = kernel32.NewProc("SetConsoleWindowInfo")
	procGetConsoleCursorInfo       = kernel32.NewProc("GetConsoleCursorInfo")
	procReadConsoleOutputCharacter = kernel32.NewProc("ReadConsoleOutputCharacterW")
	procReadConsoleOutputAttribute = kernel32.NewProc("ReadConsoleOutputAttribute")
)

type wchar uint16
type short int16
type dword uint32
type word uint16

type charInfo struct {
	buf []rune
	att []uint16
}

type consoleCursorInfo struct {
	size    dword
	visible int32
}

type coord struct {
	x, y short
}

type smallRect struct {
	left, top, right, bottom short
}

type consoleScreenBufferInfo struct {
	size              coord
	cursorPosition    coord
	attributes        word
	window            smallRect
	maximumWindowSize coord
}

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
	var csbi consoleScreenBufferInfo
	r1, _, err := procGetConsoleScreenBufferInfo.Call(
		uintptr(os.Stdout.Fd()),
		uintptr(unsafe.Pointer(&csbi)))
	if r1 == 0 && err != nil {
		return 0, 0, err
	}
	rows := csbi.window.right - csbi.window.left + 1
	cols := csbi.window.bottom - csbi.window.top + 1
	return int(cols), int(rows), nil
}

func fgToAnsi(a uint16) uint16 {
	switch a % 16 {
	case 0:
		return 30
	case 1:
		return 20
	case 2:
		return 18
	case 3:
		return 22
	case 4:
		return 17
	case 5:
		return 21
	case 6:
		return 19
	case 7:
		return 23
	case 8:
		return 30
	case 9:
		return 34
	case 10:
		return 32
	case 11:
		return 36
	case 12:
		return 31
	case 13:
		return 35
	case 14:
		return 33
	case 15:
		return 37
	}
	return 30
}

func bgToAnsi(a uint16) uint16 {
	switch a / 0x10 {
	case 0:
		return 40
	case 1:
		return 44
	case 2:
		return 42
	case 3:
		return 46
	case 4:
		return 41
	case 5:
		return 45
	case 6:
		return 43
	case 7:
		return 47
	case 8:
		return 40
	case 9:
		return 44
	case 10:
		return 42
	case 11:
		return 46
	case 12:
		return 41
	case 13:
		return 45
	case 14:
		return 43
	case 15:
		return 47
	}
	return 0
}
func getSize(sr smallRect) coord {
	return coord{sr.right - sr.left, sr.bottom - sr.top - 1}
}

func record(quit chan bool, wg *sync.WaitGroup, f io.Writer) {
	defer wg.Done()

	r := syscall.Handle(os.Stdout.Fd())

	var csbi consoleScreenBufferInfo
	r1, _, err := procGetConsoleScreenBufferInfo.Call(uintptr(r), uintptr(unsafe.Pointer(&csbi)))
	if r1 == 0 {
		fmt.Println(err)
		return
	}

	size := getSize(csbi.window)

	var oldsize coord
	var oldcurpos coord
	var oldcurvis bool
	var oldbuf []charInfo

	tm := time.NewTicker(10 * time.Millisecond)

loop:
	for {
		select {
		case <-quit:
			break loop
		case <-tm.C:
		}
		r1, _, err = procGetConsoleScreenBufferInfo.Call(uintptr(r), uintptr(unsafe.Pointer(&csbi)))
		if r1 == 0 {
			break loop
		}
		curpos := coord{
			x: csbi.cursorPosition.x - csbi.window.left,
			y: csbi.cursorPosition.y - csbi.window.top,
		}
		size = getSize(csbi.window)

		var cci consoleCursorInfo
		r1, _, err = procGetConsoleCursorInfo.Call(uintptr(r), uintptr(unsafe.Pointer(&cci)))
		if r1 == 0 {
			break loop
		}
		curvis := cci.visible != 0

		if size.x != oldsize.x || size.y != oldsize.y {
			oldbuf = []charInfo{}
		}

		l := uint32(size.x + 1)
		buf := make([]charInfo, size.y+1)
		var nr dword

		var bb bytes.Buffer
		for y := short(0); y < size.y+1; y++ {
			xy := coord{
				x: csbi.window.left,
				y: csbi.window.top + y,
			}
			cbbuf := make([]uint16, l)
			r1, _, err = procReadConsoleOutputCharacter.Call(uintptr(r), uintptr(unsafe.Pointer(&cbbuf[0])), uintptr(l), uintptr(*(*int32)(unsafe.Pointer(&xy))), uintptr(unsafe.Pointer(&nr)))
			if r1 == 0 {
				break loop
			}
			cb := utf16.Decode(cbbuf[:nr])
			buf[y].buf = cb

			ca := make([]uint16, l)
			r1, _, err = procReadConsoleOutputAttribute.Call(uintptr(r), uintptr(unsafe.Pointer(&ca[0])), uintptr(l), uintptr(*(*int32)(unsafe.Pointer(&xy))), uintptr(unsafe.Pointer(&nr)))
			if r1 == 0 {
				break loop
			}
			buf[y].att = ca[:nr]

			if len(oldbuf) > 0 {
				ob := oldbuf[y].buf
				oa := oldbuf[y].att
				diff := false
				if len(ob) != len(cb) || len(oa) != len(ca) {
					diff = true
				} else {
					for i := 0; i < len(cb); i++ {
						if ca[i] != oa[i] {
							diff = true
							break
						}
						if cb[i] != ob[i] {
							diff = true
							break
						}
					}
				}
				if !diff {
					continue
				}
			}
			a := uint16(0)
			fmt.Fprintf(&bb, "\x1b[%d;%dH\x1b[0K", y+1, 1)
			for i := 0; i < len(cb); i++ {
				if a != ca[i] {
					a = ca[i]
					fmt.Fprintf(&bb, "\x1b[%d;%dm", fgToAnsi(a), bgToAnsi(a))
				}
				fmt.Fprintf(&bb, "%s", string(cb[i]))
			}
			fmt.Fprintf(&bb, "\x1b[0m")
		}
		if oldcurpos.x != curpos.x || oldcurpos.y != curpos.y {
			fmt.Fprintf(&bb, "\x1b[%d;%dH", curpos.y+1, curpos.x+1)
		}
		if oldcurvis != curvis {
			if curvis {
				fmt.Fprintf(&bb, "\x1b[>5l")
			} else {
				fmt.Fprintf(&bb, "\x1b[>5h")
			}
		}

		if bb.Len() > 0 {
			f.Write(bb.Bytes())
			oldbuf = buf
			oldcurpos = curpos
			oldcurvis = curvis
			oldsize = size
		}
	}
}

func (p *Pty) Record(command string, w io.Writer) error {
	cmd := exec.Command("cmd", "/C", command)
	cmd.Env = append(os.Environ(), "ASCIINEMA_REC=1")
	cmd.Stdout = os.Stdout
	cmd.Stdin = os.Stdin
	cmd.Stderr = os.Stderr

	wg := new(sync.WaitGroup)
	wg.Add(1)

	sc := make(chan os.Signal, 1)
	signal.Notify(sc, os.Interrupt)
	go func() {
		for {
			<-sc
		}
	}()
	quit := make(chan bool)
	go record(quit, wg, w)

	cmd.Start()
	cmd.Wait()
	quit <- true
	wg.Wait()
	return nil
}

var out = colorable.NewColorableStdout()

func (p *Pty) Write(data []byte) error {
	_, err := out.Write(data)
	return err
}

func (p *Pty) resize(f *os.File) {
	var rows, cols int

	if isatty.IsTerminal(p.Stdout.Fd()) {
		cols, rows, _ = p.Size()
	} else {
		rows = 24
		cols = 80
	}

	rect := smallRect{
		right:  short(cols),
		bottom: short(rows),
	}
	procSetConsoleWindowInfo.Call(uintptr(f.Fd()), uintptr(int32(1)), uintptr(unsafe.Pointer(&rect)))
}
