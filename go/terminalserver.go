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
}

// CastEvent represents an event in an asciinema cast file
type CastEvent struct {
	Time   float64 `json:"time,omitempty"`
	Type   string  `json:"type,omitempty"`
	Data   string  `json:"data,omitempty"`
	PID    int     `json:"pid,omitempty"`
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
	
	fmt.Printf("Unix socket server listening on %s\n", socketPath)
	
	// WaitGroup to track active connections
	var wg sync.WaitGroup
	
	// Track terminal info by connection
	terminalInfoMutex := &sync.Mutex{}
	terminalInfo := make(map[net.Conn]*CastHeader)
	
	for {
		// Accept a connection
		conn, err := listener.Accept()
		if err != nil {
			log.Printf("Error accepting connection: %v", err)
			continue
		}
		
		// Launch a goroutine to handle this connection
		wg.Add(1)
		go handleConnection(conn, &wg, terminalInfo, terminalInfoMutex)
	}
}

func handleConnection(conn net.Conn, wg *sync.WaitGroup, terminalInfo map[net.Conn]*CastHeader, mutex *sync.Mutex) {
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
		
		// Process the line - could be JSON header or event
		if strings.HasPrefix(line, "{") {
			var header CastHeader
			if err := json.Unmarshal([]byte(line), &header); err == nil {
				// This is the header, store it for this connection
				mutex.Lock()
				terminalInfo[conn] = &header
				mutex.Unlock()
				
				fmt.Printf("[Terminal %s] Header received: v%d terminal %dx%d\n", 
					connID, header.Version, header.Term.Cols, header.Term.Rows)
			} else {
				// This might be an event with time and data
				var event CastEvent
				if err := json.Unmarshal([]byte(line), &event); err == nil {
					// Get the terminal info for this connection
					mutex.Lock()
					info := terminalInfo[conn]
					mutex.Unlock()
					
					if info != nil {
						fmt.Printf("[Terminal %s] Event time=%.6f type=%s data=%q\n", 
							connID, event.Time, event.Type, strings.TrimSpace(event.Data))
					} else {
						fmt.Printf("[Terminal %s] Event time=%.6f type=%s data=%q\n", 
							connID, event.Time, event.Type, strings.TrimSpace(event.Data))
					}
					
					// Add the connection ID as the PID
					fmt.Printf("[Terminal %s] Event time=%.6f type=%s data=%q\n", 
						connID, event.Time, event.Type, strings.TrimSpace(event.Data))
				} else {
					// Just print the line with the connection ID
					fmt.Printf("[Terminal %s] %s\n", connID, line)
				}
			}
		} else {
			// Just print the line with the connection ID
			fmt.Printf("[Terminal %s] %s\n", connID, line)
		}
	}
	
	if err := scanner.Err(); err != nil {
		log.Printf("Error reading from connection: %v", err)
	}
}