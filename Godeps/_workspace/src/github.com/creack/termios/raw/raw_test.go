package raw

import (
	"syscall"
	"testing"
	"time"

	"github.com/asciinema/asciinema/Godeps/_workspace/src/github.com/kr/pty"
)

func TestCfMakeRaw(t *testing.T) {
	termios := &Termios{
		Iflag: 0x2b02,
		Oflag: 0x1,
		Lflag: 0x5c3,
		Cflag: 0x4b00,
		Cc: [20]byte{
			0x4, 0xff, 0xff, 0xff,
			0x17, 0xff, 0x12, 0xff,
			0x3, 0x1c, 0x1a, 0x19,
			0x11, 0x13, 0x16, 0xf,
			0xff, 0xff, 0x14, 0xff,
		},
		Ispeed: 0x2580, // (9600)
		Ospeed: 0x2580, // (9600)
	}
	CfMakeRaw(termios)
	want := Termios{
		Iflag: 0x2800,
		Oflag: 0x0,
		Lflag: 0x43,
		Cflag: 0x4b00,
		Cc: [20]byte{
			0x4, 0xff, 0xff, 0xff,
			0x17, 0xff, 0x12, 0xff,
			0x3, 0x1c, 0x1a, 0x19,
			0x11, 0x13, 0x16, 0xf,
			0x1, 0x0, 0x14, 0xff,
		},
		Ispeed: 0x2580,
		Ospeed: 0x2580,
	}
	if got := *termios; want != got {
		t.Fatalf("Unexpected Raw termios.\nGot:\t %#v\nWant:\t %#v\n", got, want)
	}
}

func TestMakeRaw(t *testing.T) {
	// Create a PTY pair to play with
	master, slave, err := pty.Open()
	if err != nil {
		t.Fatal(err)
	}
	defer master.Close()
	defer slave.Close()

	// Apply the raw mode on the slave
	slaveTermios, err := MakeRaw(slave.Fd())
	if err != nil {
		t.Fatal(err)
	}

	// Make a copy of the original termios and manually apply cfmakeraw
	slaveTermiosOrig := *slaveTermios
	CfMakeRaw(&slaveTermiosOrig)

	// Retrieve the new termios on the slave after NakeRaw
	slaveTermiosRaw, err := TcGetAttr(slave.Fd())
	if err != nil {
		t.Fatal(err)
	}

	// Make sure the new termios are the one we want
	if slaveTermiosOrig != *slaveTermiosRaw {
		t.Fatalf("Unepexpected termios.\nGot:\t %#v\nWant:\t %#v\n", *slaveTermiosRaw, slaveTermiosOrig)
	}

	// Simple read/write test on the master/slave pair
	want := "hello world!"
	go master.WriteString(want)

	var (
		buf = make([]byte, 64)
		c   = make(chan struct{})
	)

	// Without raw mode, as there is no \n, read will block forever
	go func() {
		defer close(c)
		n, err := slave.Read(buf)
		if err != nil {
			t.Fatal(err)
		}
		buf = buf[:n]
	}()
	go func() {
		time.Sleep(2 * time.Second)
		buf = []byte("timeout")
		close(c)
	}()
	<-c
	if got := string(buf); got != want {
		t.Fatalf("Unexpected result.\nGot: %s\nWant: %s\n", got, want)
	}
}

func BenchmarkCfMakeRaw(b *testing.B) {
	t := &Termios{}
	for i := 0; i < b.N; i++ {
		CfMakeRaw(t)
		if t.Cc[syscall.VMIN] != 1 {
			b.Fatalf("err")
		}
	}
}
