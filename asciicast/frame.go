package asciicast

import (
	"encoding/json"
	"fmt"
	"strconv"
)

type Frame struct {
	Delay float64
	Data  []byte
}

func (f *Frame) MarshalJSON() ([]byte, error) {
	s := fmt.Sprintf(`[%.6f, "%v"]`, f.Delay, rawBytesToEscapedString(f.Data))
	return []byte(s), nil
}

func (f *Frame) UnmarshalJSON(data []byte) error {
	var x interface{}

	err := json.Unmarshal(data, &x)
	if err != nil {
		return err
	}

	f.Delay = x.([]interface{})[0].(float64)

	s := []byte(x.([]interface{})[1].(string))
	b := make([]byte, len(s))
	copy(b, s)
	f.Data = b

	return nil
}

func rawBytesToEscapedString(bytes []byte) string {
	res := make([]rune, 0)

	for _, runeValue := range string(bytes) {
		runes := []rune(strconv.Quote(string(runeValue)))
		runes = runes[1 : len(runes)-1]

		if len(runes) == 4 && runes[0] == '\\' && runes[1] == 'x' {
			runes = []rune{'\\', 'u', '0', '0', runes[2], runes[3]}
		}

		if len(runes) == 2 && runes[0] == '\\' && runes[1] == 'a' {
			runes = []rune{'\\', 'u', '0', '0', '0', '7'}
		}

		if len(runes) == 2 && runes[0] == '\\' && runes[1] == 'v' {
			runes = []rune{'\\', 'u', '0', '0', '0', 'b'}
		}

		res = append(res, runes...)
	}

	return string(res)
}
