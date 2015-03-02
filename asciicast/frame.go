package asciicast

import (
	"encoding/json"
	"fmt"
)

type Frame struct {
	Delay float64
	Data  []byte
}

func (f *Frame) MarshalJSON() ([]byte, error) {
	s, _ := json.Marshal(string(f.Data))
	json := fmt.Sprintf(`[%.6f, %s]`, f.Delay, s)
	return []byte(json), nil
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
