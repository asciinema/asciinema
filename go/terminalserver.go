package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"os"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"unicode"
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
	LastExitCode  *int
	CommandString string
	CurrentInput  string
}

func extractExitCode(data string) *int {
	re := regexp.MustCompile(`\x1b]133;D;(\d+)\x07`)
	matches := re.FindStringSubmatch(data)
	if len(matches) == 2 {
		if code, err := strconv.Atoi(matches[1]); err == nil {
			return &code
		}
	}
	return nil
}

func extractCommandString(data string) string {
	start := strings.Index(data, "\x1b]133;B\a")
	if start == -1 {
		return ""
	}
	afterB := data[start+len("\x1b]133;B\a"):]
	end := strings.Index(afterB, "\x1b[K")
	if end != -1 {
		afterB = afterB[:end]
	}
	return strings.TrimSpace(afterB)
}

func stripAnsi(str string) string {
	re := regexp.MustCompile(`\x1b\[[0-9;]*[a-zA-Z]|\x1b\][^\a]*\a`)
	return re.ReplaceAllString(str, "")
}

func isLikelyUserCommand(line string) bool {
	s := stripAnsi(line)
	s = strings.TrimSpace(s)
	return len(s) > 0 && !strings.HasPrefix(s, "\x1b]")
}

func extractCommandStringFromBuffer(buffer []string) string {
	for i := len(buffer) - 1; i >= 0; i-- {
		line := buffer[i]
		if isLikelyUserCommand(line) {
			return strings.TrimSpace(stripAnsi(line))
		}
	}
	return ""
}

func emitCommand(session *TerminalSession) {
	fmt.Printf("[debug] emitCommand: CommandString=%q, CurrentInput=%q\n", session.CommandString, session.CurrentInput)
	session.CommandString = extractCommandStringFromBuffer(session.CommandBuffer)
	fmt.Printf("[debug] emitCommand: extracted CommandString=%q\n", session.CommandString)
	fmt.Printf("[PID %d] Command finished (exit=%v):\n", session.PID, session.LastExitCode)
	fmt.Printf("  Command: %q\n", session.CommandString)
	for _, l := range session.CommandBuffer {
		fmt.Printf("    %q\n", l)
	}
	fmt.Println("---")
}

func updateCurrentInput(current *string, line string) {
	old := *current
	if line == "\b" && len(*current) > 0 {
		*current = (*current)[:len(*current)-1]
	} else if len(line) == 1 && (unicode.IsPrint(rune(line[0])) || line == " ") {
		*current += line
	} else if looksLikeFullCommand(line) {
		*current = extractCommandFromFullLine(line)
	}
	fmt.Printf("[debug] updateCurrentInput: old=%q, line=%q, new=%q\n", old, line, *current)
}

func looksLikeFullCommand(line string) bool {
	s := stripAnsi(line)
	s = strings.TrimSpace(s)
	// Heuristic: contains a space and is not just prompt or control
	return len(s) > 0 && strings.Contains(s, " ") && !strings.HasPrefix(s, "\x1b]")
}

func extractCommandFromFullLine(line string) string {
	s := stripAnsi(line)
	s = strings.TrimSpace(s)
	// Heuristic: take last word group (after prompt)
	parts := strings.Fields(s)
	if len(parts) > 0 {
		return strings.Join(parts[len(parts)-2:], " ")
	}
	return s
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
	
	sessions := make(map[int]*TerminalSession)
	
	for scanner.Scan() {
		line := scanner.Text()
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "[") {
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

			// Update current input for keystrokes and lines
			if session.State == StateCommand || session.State == StatePrompt {
				updateCurrentInput(&session.CurrentInput, data)
				// If in Command state and a full command line is echoed, update CommandString
				if session.State == StateCommand && looksLikeFullCommand(data) {
					fmt.Printf("[debug] CommandString updated in Command state: %q -> %q\n", session.CommandString, session.CurrentInput)
					session.CommandString = session.CurrentInput
				}
			}

			switch {
			case strings.Contains(data, "\x1b]133;A\a"):
				fmt.Printf("[debug] OSC 133;A: State=Prompt, CurrentInput reset\n")
				session.State = StatePrompt
				session.PromptBuffer = []string{data}
				session.CurrentInput = ""
			case strings.Contains(data, "\x1b]133;B\a"):
				fmt.Printf("[debug] OSC 133;B: State=Command, snapshot CommandString=%q\n", session.CurrentInput)
				session.State = StateCommand
				session.CommandBuffer = []string{data}
				session.CommandString = session.CurrentInput // snapshot
			case strings.Contains(data, "\x1b]133;D"):
				fmt.Printf("[debug] OSC 133;D: CommandBuffer=%v\n", session.CommandBuffer)
				session.State = StatePrompt
				session.CommandBuffer = append(session.CommandBuffer, data)
				exitCode := extractExitCode(data)
				session.LastExitCode = exitCode
				emitCommand(session)
				session.CommandBuffer = nil
				session.CommandString = ""
				session.CurrentInput = ""
			default:
				if session.State == StateCommand {
					session.CommandBuffer = append(session.CommandBuffer, data)
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