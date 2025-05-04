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
	"sync/atomic"
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
	LastExitCode  int64
	CommandString string
	CurrentInput  string
	StartTime     time.Time
	CommandId     string
}

var (
	oscDRegexp = regexp.MustCompile(`\x1b]133;D;(\d+)\x07`)
	oscPattern = regexp.MustCompile(`^\x1b\](133;[CD]|1337;RemoteHost=|1337;CurrentDir=)`)
	oscCurrentDirPattern = regexp.MustCompile(`\x1b]1337;CurrentDir=([^\x07]*)\x07`)
	regexFilters atomic.Value // []RegexFilter, protected by atomic swap
	sourceName    = "Go Terminal Server"
	sourceVersion = getBinaryModTime()
)

func init() {
	regexFilters.Store([]RegexFilter{})
}

// RegexFilter represents a step regex with optional detail extraction
// e.g. {"type": "step", "name": "Nextjs Ready", "pattern": "...", "detail": "..."}
type RegexFilter struct {
	Type    string         `json:"type"`
	Name    string         `json:"name"`
	Pattern string         `json:"pattern"`
	Detail  string         `json:"detail,omitempty"`
	Regex   *regexp.Regexp `json:"-"`
}

func looksLikeJSON(s string) bool {
	s = strings.TrimSpace(s)
	if len(s) < 2 {
		return false
	}
	return (s[0] == '{' && s[len(s)-1] == '}') || (s[0] == '[' && s[len(s)-1] == ']')
}

func extractExitCode(data string) int64 {
	matches := oscDRegexp.FindStringSubmatch(data)
	if len(matches) == 2 {
		if code, err := strconv.ParseInt(matches[1], 10, 64); err == nil {
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

type EventPayload struct {
	Event     string            `json:"event"`
	Command   string            `json:"command,omitempty"`
	CommandId string            `json:"commandId,omitempty"`
	Shell     string            `json:"shell,omitempty"`
	Username  string            `json:"username,omitempty"`
	Directory string            `json:"directory,omitempty"`
	ExitCode  *int64            `json:"exitCode,omitempty"`
	Duration  int64             `json:"duration,omitempty"`
	Name      string            `json:"name,omitempty"`
	Detail    map[string]string `json:"detail,omitempty"`
	ShouldEnd bool              `json:"shouldEnd,omitempty"`
	SourceName    string        `json:"sourceName,omitempty"`
	SourceVersion string        `json:"sourceVersion,omitempty"`
}

// getBinaryModTime returns the mod time of the running binary as RFC3339 string
func getBinaryModTime() string {
	exePath, err := os.Executable()
	if err != nil {
		return ""
	}
	fi, err := os.Stat(exePath)
	if err != nil {
		return ""
	}
	return fi.ModTime().Format(time.RFC3339)
}

// sendEvent sends a command or step event to the Electron app's local server
func sendEvent(payload EventPayload) {
	if payload.Command == "" {
		return	
	}
	payload.SourceName = sourceName
	payload.SourceVersion = sourceVersion
	url := "http://127.0.0.1:54321/"
	log.Printf("Sending %s event: %+v", payload.Event, payload)
	jsonBytes, err := json.Marshal(payload)
	if err != nil {
		log.Printf("Failed to marshal %s event: %v", payload.Event, err)
		return
	}
	go func() {
		resp, err := http.Post(url, "application/json", bytes.NewBuffer(jsonBytes))
		if err != nil {
			log.Printf("Failed to send %s event: %v", payload.Event, err)
			return
		}
		defer resp.Body.Close()
	}()
}

func regexFiltersHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}
	var incoming []struct {
		Type    string `json:"type"`
		Name    string `json:"name"`
		Pattern string `json:"pattern"`
		Detail  string `json:"detail,omitempty"`
	}
	if err := json.NewDecoder(r.Body).Decode(&incoming); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		w.Write([]byte("Invalid JSON"))
		return
	}
	fmt.Printf("Received regex filters: %v\n", incoming)
	var compiled []RegexFilter
	for _, f := range incoming {
		re, err := regexp.Compile(f.Pattern)
		if err != nil {
			w.WriteHeader(http.StatusBadRequest)
			w.Write([]byte("Invalid regex: " + f.Pattern))
			return
		}
		compiled = append(compiled, RegexFilter{Type: f.Type, Name: f.Name, Pattern: f.Pattern, Detail: f.Detail, Regex: re})
	}
	regexFilters.Store(compiled)
	w.WriteHeader(http.StatusOK)
	w.Write([]byte("OK"))
}

// matchStepEvent returns the name and detail (if any) of the first matching regex filter, or empty strings
func matchStepEvent(line string) (string, map[string]string) {
	filters := regexFilters.Load().([]RegexFilter)
	for _, f := range filters {
		if f.Type != "step" { continue }
		if m := f.Regex.FindStringSubmatch(line); m != nil {
			fmt.Printf("Found match for %s: %v\n", f.Name, m)
			result := map[string]string{}
			if len(f.Regex.SubexpNames()) > 1 {
				for i, name := range f.Regex.SubexpNames() {
					if i != 0 && name != "" {
						result[name] = m[i]
					}
				}
			}
			return f.Name, result
		}
	}
	return "", nil
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
	
	// Start HTTP server for regex filters
	go func() {
		http.HandleFunc("/regexfilters", regexFiltersHandler)
		log.Fatal(http.ListenAndServe(":54322", nil))
	}()
	
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

			switch {
			case strings.Contains(data, "\x1b]133;B\a"):
				cmd := extractCommandFromOSC133B(data)
				if cmd != "" {
					fmt.Printf("[COMMAND START] Just entered: %q\n", cmd)
					session.CommandString = cmd
					session.State = StateCommand // Set state to Command
					session.CommandBuffer = nil  // Clear previous buffer
					session.StartTime = time.Now()
					session.CommandId = fmt.Sprintf("%dN-%d", time.Now().Unix(), pidInt)
					// Send start event (no exitCode)
					sendEvent(EventPayload{
						Event:     "start",
						Command:   cmd,
						CommandId: session.CommandId,
						Shell:     shell,
						Username:  username,
						Directory: directory,
					})
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
				// Send end event (always send exitCode as pointer)
				endExitCode := exitCode
				duration := int64(100) // Default 100ms
				if !session.StartTime.IsZero() {
					duration = time.Since(session.StartTime).Milliseconds()
				}
				sendEvent(EventPayload{
					Event:     "end",
					Command:   session.CommandString,
					CommandId: session.CommandId,
					Shell:     shell,
					Username:  username,
					Directory: directory,
					ExitCode:  &endExitCode,
					Duration:  duration,
				})
				session.CommandBuffer = nil
				session.CommandString = ""
				session.StartTime = time.Time{}
				session.CurrentInput = ""
			default:
				if session.State == StateCommand {
					if isRealOutput(data) {
						stepName, detail := matchStepEvent(data)
						if stepName != "" {
							if detail != nil && len(detail) > 0 {
								detailJson, _ := json.Marshal(detail)
								fmt.Printf("[step %s] %q detail=%s\n", stepName, data, detailJson)
							} else {
								fmt.Printf("[step %s] %q\n", stepName, data)
							}
							// Send step event to Electron app (no exitCode)
							sendEvent(EventPayload{
								Event:     "step",
								Command:   session.CommandString,
								CommandId: session.CommandId,
								Shell:     shell,
								Username:  username,
								Directory: directory,
								Name:      stepName,
								Detail:    detail,
								ShouldEnd: false,
							})
						}
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