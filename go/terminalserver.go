package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"os"
	"strings"
	"sync"
)

// TermInfo represents terminal metadata
type TermInfo struct {
	Cols    int    `json:"cols"`
	Rows    int    `json:"rows"`
	Type    string `json:"type"`
	Version string `json:"version"`
	Theme   map[string]interface{} `json:"theme"`
}

// CastHeader represents the header of an asciinema cast file
type CastHeader struct {
	Version   int      `json:"version"`
	Term      TermInfo `json:"term"`
	Timestamp int64    `json:"timestamp"`
	Env       map[string]string `json:"env"`
	ChildPID  int        `json:"child_pid"`
}

// CastEvent represents an event in an asciinema cast file
type CastEvent struct {
	Time   float64 `json:"time,omitempty"`
	Type   string  `json:"type,omitempty"`
	Data   string  `json:"data,omitempty"`
	PID    int     `json:"pid,omitempty"`
}

// CommandBuffer holds lines for a command session
type CommandBuffer struct {
	Lines []string
	Active bool
}

func main() {
	socketPath := "/tmp/test.sock"
	
	// Remove socket if it already exists
	if err := os.RemoveAll(socketPath); err != nil {
		log.Fatal("Error removing existing socket:", err)
	}
	
	// Create the socket
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		log.Fatal("Error creating socket:", err)
	}
	defer listener.Close()
	
	fmt.Printf("2Unix socket server listening on %s\n", socketPath)
	
	// WaitGroup to track active connections
	var wg sync.WaitGroup
	
	// Track terminal info by connection
	terminalInfoMutex := &sync.Mutex{}
	terminalInfo := make(map[net.Conn]*CastHeader)
	
	// Track command buffers by child_pid
	commandBuffers := make(map[int]*CommandBuffer)
	
	for {
		// Accept a connection
		conn, err := listener.Accept()
		if err != nil {
			log.Printf("Error accepting connection: %v", err)
			continue
		}
		
		// Launch a goroutine to handle this connection
		wg.Add(1)
		go handleConnection(conn, &wg, terminalInfo, terminalInfoMutex, commandBuffers)
	}
}

func handleConnection(conn net.Conn, wg *sync.WaitGroup, terminalInfo map[net.Conn]*CastHeader, mutex *sync.Mutex, commandBuffers map[int]*CommandBuffer) {
	defer func() {
		conn.Close()
		
		// Remove terminal info when connection closes
		mutex.Lock()
		delete(terminalInfo, conn)
		mutex.Unlock()
		
		wg.Done()
	}()
	
	// Read data from the connection
	scanner := bufio.NewScanner(conn)
	
	// Generate a unique ID for this connection
	connID := fmt.Sprintf("%p", conn)
	fmt.Printf("New connection established: %s\n", connID)
	
	for scanner.Scan() {
		line := scanner.Text()
		trimmed := strings.TrimSpace(line)
		// fmt.Printf("[debug] Raw line: %q\n", line)
		// fmt.Printf("[debug] Trimmed line: %q\n", trimmed)
		// fmt.Printf("[debug] HasPrefix [ : %v\n", strings.HasPrefix(trimmed, "["))
		if strings.HasPrefix(trimmed, "[") {
			var arr []interface{}
			err := json.Unmarshal([]byte(trimmed), &arr)
			if err != nil {
				// fmt.Printf("[debug] JSON unmarshal error: %v\n", err)
			} else {
				// fmt.Printf("[debug] Unmarshaled array: %#v\n", arr)
				if len(arr) >= 4 {
					// fmt.Printf("[debug] arr[0] type: %T, arr[1] type: %T, arr[2] type: %T, arr[3] type: %T\n", arr[0], arr[1], arr[2], arr[3])
					pid, okPid := arr[3].(float64)
					data, okData := arr[2].(string)
					// fmt.Printf("[debug] okPid: %v, pid: %v | okData: %v, data: %v\n", okPid, pid, okData, data)
					if okPid && okData {
						pidInt := int(pid)
						fmt.Printf("[%d] %q\n", pidInt, data)
						// Detect OSC 133;B (start) and OSC 133;D (end)
						if strings.Contains(data, "\u001b]133;B\u0007") {
							commandBuffers[pidInt] = &CommandBuffer{Active: true}
						} else if strings.Contains(data, "\u001b]133;D\u0007") {
							if buf, ok := commandBuffers[pidInt]; ok && buf.Active {
								fmt.Printf("[PID %d] Command output:\n%s\n---\n", pidInt, strings.Join(buf.Lines, "\n"))
								buf.Active = false
							}
						} else {
							if buf, ok := commandBuffers[pidInt]; ok && buf.Active {
								buf.Lines = append(buf.Lines, data)
							}
						}
					} else {
						fmt.Printf("[unknown] %s\n", trimmed)
					}
				} else {
					fmt.Printf("[unknown] %s\n", trimmed)
				}
			}
		} else {
			// fmt.Printf("[debug] Line does not start with [: %q\n", trimmed)
			fmt.Printf("[unknown] %s\n", trimmed)
		}
	}
	
	if err := scanner.Err(); err != nil {
		log.Printf("Error reading from connection: %v", err)
	}
}