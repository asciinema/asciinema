package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"
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
	Username  string     `json:"username,omitempty"`
	Directory string     `json:"directory,omitempty"`
	Shell     string     `json:"shell,omitempty"`
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

type SessionState string

const (
	StateIdle    SessionState = "Idle"
	StatePrompt  SessionState = "Prompt"
	StateCommand SessionState = "Command"
)

type TerminalSession struct {
	PID           int
	State         SessionState
	CommandBuffer []string
	PromptBuffer  []string
	LastExitCode  int
	CommandString string
	CurrentInput  string
	StartTime     time.Time
}

var (
	oscDRegexp = regexp.MustCompile(`\x1b]133;D;(\d+)\x07`)
	oscPattern = regexp.MustCompile(`^\x1b\](133;[CD]|1337;RemoteHost=|1337;CurrentDir=)`)
	oscCurrentDirPattern = regexp.MustCompile(`\x1b]1337;CurrentDir=([^\x07]*)\x07`)
)

func looksLikeJSON(s string) bool {
	s = strings.TrimSpace(s)
	if len(s) < 2 {
		return false
	}
	return (s[0] == '{' && s[len(s)-1] == '}') || (s[0] == '[' && s[len(s)-1] == ']')
}

func extractExitCode(data string) int {
	matches := oscDRegexp.FindStringSubmatch(data)
	if len(matches) == 2 {
		if code, err := strconv.Atoi(matches[1]); err == nil {
			return code
		}
	}
	// If not found or error, return -1 (should not happen if always present)
	return -1
}

// Add this function for extracting the command from OSC 133;B
func extractCommandFromOSC133B(line string) string {
	start := strings.Index(line, "\x1b]133;B\a")
	if start == -1 {
		return ""
	}
	afterB := line[start+len("\x1b]133;B\a"):]
	end := strings.Index(afterB, "\x1b[K")
	if end != -1 {
		afterB = afterB[:end]
	}
	return strings.TrimSpace(afterB)
}

// Add a helper to check if a line is real output (not just OSC/control)
func isRealOutput(data string) bool {
	// Skip OSC 133;C, 133;D, 1337;RemoteHost, 1337;CurrentDir, etc
	return !oscPattern.MatchString(data) && strings.TrimSpace(data) != ""
}

// sendCommandEvent sends a command lifecycle event to the Electron app's local server
func sendCommandEvent(event string, command string, commandId string, shell string, username string, directory string, exitCode int, duration int64) {
	if command == "" { // Don't send events for empty commands
		return
	}
	url := "http://127.0.0.1:54321/"
	msg := map[string]interface{}{
		"event": event,
		"command": command,
		"username": username,
		"directory": directory,
		"commandId": commandId,
		"shell": shell,
	}
	if event == "end" {
		msg["exitCode"] = exitCode
		msg["duration"] = duration
	}
	jsonBytes, err := json.Marshal(msg)
	if err != nil {
		log.Printf("Failed to marshal command event: %v", err)
		return
	}
	go func() {
		resp, err := http.Post(url, "application/json", bytes.NewBuffer(jsonBytes))
		if err != nil {
			log.Printf("Failed to send command event: %v", err)
			return
		}
		defer resp.Body.Close()
	}()
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
	
	var headerParsed bool
	var header CastHeader
	var username, directory, shell string

	sessions := make(map[int]*TerminalSession)
	
	for scanner.Scan() {
		line := scanner.Text()
		trimmed := strings.TrimSpace(line)
		if !headerParsed && looksLikeJSON(trimmed) && trimmed[0] == '{' {
			err := json.Unmarshal([]byte(trimmed), &header)
			if err == nil {
				headerParsed = true
				mutex.Lock()
				terminalInfo[conn] = &header
				mutex.Unlock()
				username = header.Username
				directory = header.Directory
				shell = header.Shell
				fmt.Printf("[header] username=%q directory=%q shell=%q\n", username, directory, shell)
				continue
			}
		}
		if looksLikeJSON(trimmed) && trimmed[0] == '[' {
			var arr []interface{}
			err := json.Unmarshal([]byte(trimmed), &arr)
			if err != nil || len(arr) < 4 {
				fmt.Printf("[unknown] %s\n", trimmed)
				continue
			}
			pid, okPid := arr[3].(float64)
			data, okData := arr[2].(string)
			if !okPid || !okData {
				fmt.Printf("[unknown] %s\n", trimmed)
				continue
			}
			pidInt := int(pid)
			session, exists := sessions[pidInt]
			if !exists {
				session = &TerminalSession{PID: pidInt, State: StateIdle}
				sessions[pidInt] = session
			}

			// [raw <pid>] logging
			fmt.Printf("[raw %d] %q\n", pidInt, data)

			// Check for OSC 1337;CurrentDir= in the data field
			if matches := oscCurrentDirPattern.FindStringSubmatch(data); len(matches) == 2 {
				directory = matches[1]
				fmt.Printf("[directory changed] %s\n", directory)
			}

			commandId := fmt.Sprintf("%dN-%d", time.Now().Unix(), pidInt)
			switch {
			case strings.Contains(data, "\x1b]133;B\a"):
				cmd := extractCommandFromOSC133B(data)
				if cmd != "" {
					fmt.Printf("[COMMAND START] Just entered: %q\n", cmd)
					session.CommandString = cmd
					session.State = StateCommand // Set state to Command
					session.CommandBuffer = nil  // Clear previous buffer
					session.StartTime = time.Now()
					// Send start event
					sendCommandEvent("start", cmd, commandId, shell, username, directory, 0, 0)
				}
			case strings.Contains(data, "\x1b]133;D"):
				fmt.Printf("[debug] OSC 133;D: CommandBuffer=%v\n", session.CommandBuffer)
				session.State = StatePrompt
				// Do not append OSC 133;D to CommandBuffer, just handle exit code
				exitCode := extractExitCode(data)
				session.LastExitCode = exitCode
				// Print command, exit code, and output directly
				fmt.Printf("[COMMAND END] PID %d, exit=%d\n", session.PID, session.LastExitCode)
				fmt.Printf("  Command: %q\n", session.CommandString)
				for _, l := range session.CommandBuffer {
					fmt.Printf("    %q\n", l)
				}
				fmt.Println("---")
				// Send end event
				duration := int64(100) // Default 100ms
				if !session.StartTime.IsZero() {
					duration = time.Since(session.StartTime).Milliseconds()
				}
				sendCommandEvent("end", session.CommandString, commandId, shell, username, directory, exitCode, duration)
				session.CommandBuffer = nil
				session.CommandString = ""
				session.StartTime = time.Time{}
				session.CurrentInput = ""
			default:
				if session.State == StateCommand {
					if isRealOutput(data) {
						session.CommandBuffer = append(session.CommandBuffer, data)
					}
				} else if session.State == StatePrompt {
					session.PromptBuffer = append(session.PromptBuffer, data)
				}
			}
		} else {
			fmt.Printf("[unknown] %s\n", trimmed)
		}
	}
	
	if err := scanner.Err(); err != nil {
		log.Printf("Error reading from connection: %v", err)
	}
}