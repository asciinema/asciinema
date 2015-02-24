package asciicast_test

import (
	"encoding/json"
	"testing"

	"github.com/asciinema/asciinema-cli/asciicast"
)

func TestFrame_MarshalJSON(t *testing.T) {
	var tests = []struct {
		delay    float64
		data     []byte
		expected string
	}{
		{
			4.906e-06,
			[]byte{0x1b, 0x5b, 0x31, 0x6d, 0x25, 0x1b, 0x5b, 0x32, 0x33, 0x6d, 0x1b, 0x5b, 0x30, 0x6d, 0x0d, 0x20, 0x0d, 0xc5, 0x82, 0x07, 0x0b, 0x85, 0x61},
			`{"test":[0.000005,"\u001b[1m%\u001b[23m\u001b[0m\r \rł\u0007\u000b�a"]}`,
		},
		{
			12.345,
			[]byte{0xe2, 0x8c, 0x98, 0x6c, 0x25, 0x50, 0x93, 0xe8, 0xd4, 0x6a, 0x03, 0xbe, 0xf3, 0xfe, 0xc3, 0x45, 0xee, 0x87, 0xca, 0x6b, 0x92, 0xa6, 0xa7, 0x8f, 0xb8, 0x85, 0xd0, 0x07, 0x91, 0x9b, 0x91, 0x45, 0x2f, 0x1c, 0xc8, 0xb3, 0x26, 0x96, 0xfa, 0x22, 0x8e, 0x3f, 0x12, 0x64, 0xcf, 0xf0, 0xe4, 0x01, 0x71, 0xee, 0x65, 0x6e, 0x4a, 0x7a, 0x81, 0x3f, 0x2f, 0x84, 0x3f, 0xc4, 0x27, 0x2d, 0xf5, 0x35, 0x34, 0x02, 0x6c},
			`{"test":[12.345000,"⌘l%P���j\u0003����E���k�������\u0007���E/\u001cȳ\u0026��\"�?\u0012d���\u0001q�enJz�?/�?�'-�54\u0002l"]}`,
		},
	}

	for _, test := range tests {
		frame := asciicast.Frame{
			Delay: test.delay,
			Data:  test.data,
		}

		data := map[string]*asciicast.Frame{
			"test": &frame,
		}

		bytes, err := json.Marshal(data)
		if err != nil {
			t.Errorf("got error: %v", err)
			return
		}

		if string(bytes) != test.expected {
			t.Errorf(`expected: %v, got: %v`, test.expected, string(bytes))
			return
		}
	}
}

func TestFrame_UnmarshalJSON(t *testing.T) {
	var f asciicast.Frame

	err := json.Unmarshal([]byte(`[1.23, "\u001b[0mżółć"]`), &f)
	if err != nil {
		t.Errorf("got error: %v", err)
		return
	}

	if f.Delay != 1.23 {
		t.Errorf(`expected 1.23, got %v`, f.Delay)
		return
	}

	expected := "\u001b[0mżółć"
	if string(f.Data) != expected {
		t.Errorf(`expected "%v", got "%v"`, expected, string(f.Data))
		return
	}
}
