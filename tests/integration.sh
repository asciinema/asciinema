#!/usr/bin/env bash

set -eEuo pipefail -o errtrace

# Colors for output (disabled if no TTY or NO_COLOR set)
if [[ ! -t 1 ]] || [[ -n "${NO_COLOR:-}" ]]; then
    RED=""
    GREEN=""
    YELLOW=""
    BLUE=""
    NC=""
else
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
fi

# Test tracking
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
log_info() {
    printf "%b\n" "${BLUE}INFO:${NC} $*"
}

log_success() {
    printf "%b\n" "${GREEN}PASS:${NC} $*"
    ((TESTS_PASSED++))
}

log_error() {
    printf "%b\n" "${RED}FAIL:${NC} $*"
    ((TESTS_FAILED++))
}

log_warning() {
    printf "%b\n" "${YELLOW}WARN:${NC} $*"
}

assert_exit_code() {
    local expected=$1
    local actual=$2
    local test_name=$3
    
    ((TESTS_RUN++))
    if [[ $actual -eq $expected ]]; then
        log_success "$test_name - exit code $actual"
    else
        log_error "$test_name - expected exit code $expected, got $actual"
        return 1
    fi
}

assert_file_exists() {
    local file=$1
    local test_name=$2
    
    ((TESTS_RUN++))
    if [[ -f "$file" ]]; then
        log_success "$test_name - file exists: $file"
    else
        log_error "$test_name - file does not exist: $file"
        return 1
    fi
}

assert_file_not_empty() {
    local file=$1
    local test_name=$2
    
    ((TESTS_RUN++))
    if [[ -s "$file" ]]; then
        log_success "$test_name - file not empty: $file"
    else
        log_error "$test_name - file is empty: $file"
        return 1
    fi
}

assert_output_contains() {
    local expected=$1
    local output=$2
    local test_name=$3
    
    ((TESTS_RUN++))
    if echo "$output" | grep -qF -- "$expected"; then
        log_success "$test_name - output contains: $expected"
    else
        log_error "$test_name - output missing: $expected"
        log_error "Actual output: $output"
        return 1
    fi
}

assert_file_contains() {
    local expected=$1
    local file=$2
    local test_name=$3
    
    ((TESTS_RUN++))
    if grep -qF -- "$expected" "$file"; then
        log_success "$test_name - file contains: $expected"
    else
        log_error "$test_name - file missing: $expected"
        return 1
    fi
}

# SETUP
setup() {
    log_info "Setting up test environment..."
    
    ASCIINEMA_CONFIG_HOME="$(
        mktemp -d 2>/dev/null || mktemp -d -t asciinema-config-home
    )"

    ASCIINEMA_STATE_HOME="$(
        mktemp -d 2>/dev/null || mktemp -d -t asciinema-state-home
    )"

    ASCIINEMA_GEN_DIR="$(
        mktemp -d 2>/dev/null || mktemp -d -t asciinema-gen-dir
    )"

    export ASCIINEMA_CONFIG_HOME ASCIINEMA_STATE_HOME ASCIINEMA_GEN_DIR
    export ASCIINEMA_SERVER_URL=https://asciinema.example.com

    TMP_DATA_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t asciinema-data-dir)"
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    FIXTURES="$SCRIPT_DIR/casts"
    ASCIINEMA_BIN="$SCRIPT_DIR/../target/integration-test/asciinema"

    trap 'cleanup' EXIT

    log_info "Building release binary..."
    cargo build --profile=integration-test --locked

    # disable notifications
    printf "[notifications]\nenabled = false\n" >> "${ASCIINEMA_CONFIG_HOME}/config.toml"
    
    log_info "Setup complete"
}

cleanup() {
    log_info "Cleaning up..."
    rm -rf "${ASCIINEMA_CONFIG_HOME:-}" "${ASCIINEMA_STATE_HOME:-}" "${ASCIINEMA_GEN_DIR:-}" "${TMP_DATA_DIR:-}"
}

# Test runner function
run_test() {
    local test_name="$1"
    shift
    
    if [[ -z "${TEST:-}" || "${TEST:-}" == "$test_name" ]]; then
        echo
        echo "#################### TEST $test_name ####################"
        "$@" || true  # Don't exit on test failure
    fi
}

# TEST FUNCTIONS

test_help() {
    log_info "Testing help command..."
    
    # Test short help
    local output rc
    if output=$("$ASCIINEMA_BIN" -h 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "help short flag"
    assert_output_contains "Terminal session recorder" "$output" "help content"
    assert_output_contains "Commands:" "$output" "help shows commands"
    
    # Test long help
    if output=$("$ASCIINEMA_BIN" --help 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "help long flag"
    assert_output_contains "Terminal session recorder" "$output" "help content"
    
    # Test help subcommand
    if output=$("$ASCIINEMA_BIN" help 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "help subcommand"
    assert_output_contains "Terminal session recorder" "$output" "help subcommand content"
}

test_version() {
    log_info "Testing version command..."
    
    # Test short version
    local output rc
    if output=$("$ASCIINEMA_BIN" -V 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "version short flag"
    assert_output_contains "asciinema" "$output" "version output format"
    
    # Test long version
    if output=$("$ASCIINEMA_BIN" --version 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "version long flag"
    assert_output_contains "asciinema" "$output" "version output format"
}

test_auth() {
    log_info "Testing auth command..."
    
    # Test auth command (should handle offline gracefully)
    local output rc
    if output=$("$ASCIINEMA_BIN" auth 2>&1); then rc=0; else rc=$?; fi
    
    # Auth should complete without hanging and show expected message
    assert_exit_code 0 "$rc" "auth"
    assert_output_contains "Open the following URL in a web browser" "$output" "auth command output"
}

test_record() {
    log_info "Testing record command..."
    
    # Test basic recording
    local file1="$TMP_DATA_DIR/record_basic.cast"
    local rc
    if "$ASCIINEMA_BIN" record --headless --command 'echo "hello world"' --return "$file1"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "record basic"
    assert_file_contains '"o",' "$file1" "record output event"
    assert_file_contains 'hello world' "$file1" "record output content"
    
    # Test different formats
    local file2="$TMP_DATA_DIR/record_v2.cast"
    if "$ASCIINEMA_BIN" record --headless --command 'echo "test v2"' --output-format asciicast-v2 --return "$file2"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "record v2 format"
    assert_file_not_empty "$file2" "record v2 format"
    
    local file3="$TMP_DATA_DIR/record_v3.cast"
    if "$ASCIINEMA_BIN" record --headless --command 'echo "test v3"' --output-format asciicast-v3 --return "$file3"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "record v3 format"
    assert_file_not_empty "$file3" "record v3 format"
    
    # Test raw format
    local file4="$TMP_DATA_DIR/record_raw.raw"
    if "$ASCIINEMA_BIN" record --headless --command 'echo "test raw"' --output-format raw --return "$file4"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "record raw format"
    assert_file_not_empty "$file4" "record raw format"
    
    # Test txt format
    local file5="$TMP_DATA_DIR/record_txt.txt"
    if "$ASCIINEMA_BIN" record --headless --command 'echo "test txt"' --output-format txt --return "$file5"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "record txt format"
    assert_file_not_empty "$file5" "record txt format"
    
    # Test return flag with failure
    local file6="$TMP_DATA_DIR/record_fail.cast"
    if "$ASCIINEMA_BIN" record --headless --command 'exit 42' --return "$file6"; then rc=0; else rc=$?; fi
    assert_exit_code 42 "$rc" "record return flag with failure"
    assert_file_not_empty "$file6" "record failure"
    
    # Test append mode
    local file7="$TMP_DATA_DIR/record_append.cast"
    if "$ASCIINEMA_BIN" record --headless --command 'echo "first"' --return "$file7"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "record append setup"
    if "$ASCIINEMA_BIN" record --headless --command 'echo "second"' --append --return "$file7"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "record append"
    assert_file_contains 'first' "$file7" "record append first content"
    assert_file_contains 'second' "$file7" "record append second content"
    
    # Test idle time limits
    local file8="$TMP_DATA_DIR/record_idle.cast"
    if "$ASCIINEMA_BIN" record --headless --command 'bash -c "echo start; sleep 2; echo end"' --idle-time-limit 1 --return "$file8"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "record idle time limit"
    assert_file_not_empty "$file8" "record idle time limit"
}

test_play() {
    log_info "Testing play command..."
    local fixture="$FIXTURES/minimal-v2.cast"

    # Playback requires a controlling terminal and access to /dev/tty.
    if ! tty -s || [[ ! -r /dev/tty ]] || [[ ! -w /dev/tty ]]; then
        log_warning "Skipping play tests: controlling terminal is not available"
        return
    fi

    # Test playback from regular file path
    local output rc
    if output=$(timeout --foreground 10s "$ASCIINEMA_BIN" play --speed 1000 "$fixture" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "play from file"
    assert_output_contains "Replaying session from $fixture" "$output" "play file start message"
    assert_output_contains "Playback ended" "$output" "play file end message"

    # Test playback from stdin ("-")
    if output=$(timeout --foreground 10s "$ASCIINEMA_BIN" play --speed 1000 - < "$fixture" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "play from stdin"
    assert_output_contains "Replaying session from -" "$output" "play stdin start message"
    assert_output_contains "Playback ended" "$output" "play stdin end message"
}

test_stream() {
    log_info "Testing stream command..."
    
    # Test local streaming
    timeout 10s "$ASCIINEMA_BIN" stream --headless --local 127.0.0.1:8081 --command 'bash -c "echo streaming test; sleep 3; echo done"' --return &
    local stream_pid=$!
    
    # Wait a moment for server to start
    sleep 1
    
    # Test if HTTP server is responding and serving the player
    local curl_output rc
    if curl_output=$(curl -fsS "http://127.0.0.1:8081" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "stream server responding"
    assert_output_contains "AsciinemaPlayer" "$curl_output" "stream server AsciinemaPlayer content"
    
    # Clean up
    kill $stream_pid 2>/dev/null || true
    wait $stream_pid 2>/dev/null || true
}

test_session() {
    log_info "Testing session command..."
    
    # Test session with file output
    local file1="$TMP_DATA_DIR/session_basic.cast"
    local rc
    if "$ASCIINEMA_BIN" session --headless --output-file "$file1" --command 'echo "session test"' --return; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "session basic"
    assert_file_contains 'session test' "$file1" "session output content"
    
    # Test session with return flag failure
    local file2="$TMP_DATA_DIR/session_fail.cast"
    if "$ASCIINEMA_BIN" session --headless --output-file "$file2" --command 'exit 13' --return; then rc=0; else rc=$?; fi
    assert_exit_code 13 "$rc" "session return flag with failure"
    assert_file_contains '"x", "13"' "$file2" "session exit event"
    
    # Test session with local streaming + file output
    local file3="$TMP_DATA_DIR/session_stream.cast"
    timeout 8s "$ASCIINEMA_BIN" session --headless --output-file "$file3" --stream-local 127.0.0.1:8081 --command 'bash -c "echo stream session; sleep 3; echo done"' --return &
    local session_pid=$!
    
    # Wait a moment for server to start
    sleep 1
    
    # Test if both file and HTTP server work
    local curl_output
    if curl_output=$(curl -fsS "http://127.0.0.1:8081" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "stream server responding"
    assert_output_contains "AsciinemaPlayer" "$curl_output" "session streaming server AsciinemaPlayer content"
    
    # Clean up and check file
    kill $session_pid 2>/dev/null || true
    wait $session_pid 2>/dev/null || true
    
    if [[ -f "$file3" ]]; then
        assert_file_contains 'stream session' "$file3" "session output content"
    fi
    
    # Test different output formats
    local file4="$TMP_DATA_DIR/session_v2.cast"
    if "$ASCIINEMA_BIN" session --headless --output-file "$file4" --output-format asciicast-v2 --command 'echo "session v2"' --return; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "session v2 format"
    assert_file_contains 'session v2' "$file4" "session output content"
    
    # Test append mode
    local file5="$TMP_DATA_DIR/session_append.cast"
    if "$ASCIINEMA_BIN" session --headless --output-file "$file5" --command 'echo "first session"' --return; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "session append setup"
    if "$ASCIINEMA_BIN" session --headless --output-file "$file5" --append --command 'echo "second session"' --return; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "session append"
    assert_file_contains 'first session' "$file5" "session append first content"
    assert_file_contains 'second session' "$file5" "session append second content"
}

test_cat() {
    log_info "Testing cat command..."
    
    # Create test recordings first
    local file1="$TMP_DATA_DIR/cat_input1.cast"
    local file2="$TMP_DATA_DIR/cat_input2.cast"
    local rc
    
    if "$ASCIINEMA_BIN" record --headless --command 'echo "first recording"' --return "$file1"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat setup first recording"
    
    if "$ASCIINEMA_BIN" record --headless --command 'echo "second recording"' --return "$file2"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat setup second recording"
    
    # Test concatenation to stdout
    local output
    if output=$("$ASCIINEMA_BIN" cat "$file1" "$file2" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat concatenate"
    assert_output_contains 'first recording' "$output" "cat first content"
    assert_output_contains 'second recording' "$output" "cat second content"
    
    # Test with different format inputs (using fixtures, v2+v3 only since v1 can't be concatenated)
    if output=$("$ASCIINEMA_BIN" cat "$FIXTURES/minimal-v2.cast" "$FIXTURES/minimal-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat mixed formats"
    assert_output_contains '"version":' "$output" "cat mixed formats output"
}

test_convert() {
    log_info "Testing convert command..."
    
    # Test v1 to v3 conversion
    local file1="$TMP_DATA_DIR/convert_v1_to_v3.cast"
    local rc
    if "$ASCIINEMA_BIN" convert "$FIXTURES/minimal-v1.json" "$file1"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "convert v1 to v3"
    assert_file_contains '"version":3' "$file1" "convert v1 to v3 version"
    
    # Test v2 to v3 conversion
    local file2="$TMP_DATA_DIR/convert_v2_to_v3.cast"
    if "$ASCIINEMA_BIN" convert "$FIXTURES/minimal-v2.cast" "$file2"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "convert v2 to v3"
    assert_file_contains '"version":3' "$file2" "convert v2 to v3 version"
    
    # Test to raw format
    local file3="$TMP_DATA_DIR/convert_to_raw.raw"
    if "$ASCIINEMA_BIN" convert --output-format raw "$FIXTURES/minimal-v2.cast" "$file3"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "convert to raw"
    assert_file_exists "$file3" "convert to raw output"
    
    # Test to txt format
    local file4="$TMP_DATA_DIR/convert_to_txt.txt"
    if "$ASCIINEMA_BIN" convert --output-format txt "$FIXTURES/minimal-v2.cast" "$file4"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "convert to txt"
    assert_file_exists "$file4" "convert to txt output"
    
    # Test output to stdout
    local output
    if output=$("$ASCIINEMA_BIN" convert "$FIXTURES/minimal-v2.cast" - 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "convert to stdout"
    assert_output_contains '"version":3' "$output" "convert stdout version"
    
    # Test overwrite behavior
    local file5="$TMP_DATA_DIR/convert_overwrite.cast"
    echo "existing content" > "$file5"
    if "$ASCIINEMA_BIN" convert --overwrite "$FIXTURES/minimal-v2.cast" "$file5"; then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "convert overwrite"
    assert_file_contains '"version":3' "$file5" "convert overwrite content"
}

test_ls_frames() {
    log_info "Testing ls-frames command..."

    local output
    local rc

    # Test table format (default)
    if output=$("$ASCIINEMA_BIN" ls-frames "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "ls-frames table"
    assert_output_contains 'DELTA-CELLS' "$output" "ls-frames table header"
    assert_output_contains 'marker  step 1' "$output" "ls-frames table marker"
    assert_output_contains 'resize  8x3' "$output" "ls-frames table resize"

    # Test csv format
    if output=$("$ASCIINEMA_BIN" ls-frames --format csv "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "ls-frames csv"
    assert_output_contains 'frame,time,delta_time,delta_cells,type,detail' "$output" "ls-frames csv header"
    assert_output_contains '1,0.500000,0.500000,3,output,' "$output" "ls-frames csv row"

    # Test json format
    if output=$("$ASCIINEMA_BIN" ls-frames --format json "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "ls-frames json"
    assert_output_contains '"delta_cells"' "$output" "ls-frames json fields"
    assert_output_contains '"label":"step 1"' "$output" "ls-frames json marker label"

    # Test with a v2 recording
    if output=$("$ASCIINEMA_BIN" ls-frames "$FIXTURES/minimal-v2.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "ls-frames v2 input"

    # A recording with degenerate terminal dimensions must not crash
    local zero_dim="$TMP_DATA_DIR/ls_frames_zero_dim.cast"
    printf '%s\n' '{"version":3,"term":{"cols":0,"rows":0}}' '[0.1,"o","hi"]' > "$zero_dim"
    if output=$("$ASCIINEMA_BIN" ls-frames "$zero_dim" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "ls-frames zero dimensions"

    # A malicious marker label must not inject escape sequences into the table
    local evil="$TMP_DATA_DIR/ls_frames_evil.cast"
    printf '%s\n' '{"version":3,"term":{"cols":20,"rows":3}}' '[0.1,"o","hi"]' '[0.2,"m","\u001b]0;x\u0007evil"]' > "$evil"
    if output=$("$ASCIINEMA_BIN" ls-frames "$evil" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "ls-frames escaped marker"
    assert_output_contains 'u{1b}]0;x' "$output" "ls-frames marker label escaped"
}

test_cat_frames() {
    log_info "Testing cat-frames command..."

    local output
    local rc

    # Test frame number selection
    if output=$("$ASCIINEMA_BIN" cat-frames --frame 1 "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames frame"
    assert_output_contains '--- frame 1 @ 0.500000 ---' "$output" "cat-frames frame header"

    # Test frame range selection with plain text output
    if output=$("$ASCIINEMA_BIN" cat-frames --frame 1,4-5 --no-escapes "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames frame range"
    assert_output_contains 'red text' "$output" "cat-frames frame range content"
    assert_output_contains '(resize: 8x3)' "$output" "cat-frames resize annotation"

    # Test time selection
    if output=$("$ASCIINEMA_BIN" cat-frames --time 2.5 --no-escapes "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames time"
    assert_output_contains '--- frame 3 @ 2.000000 (marker: step 1) ---' "$output" "cat-frames time header"

    # Test time range selection with json output
    if output=$("$ASCIINEMA_BIN" cat-frames --time 0.7-2.5 --format json "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames time range json"
    assert_output_contains '"text":["red text","","",""]' "$output" "cat-frames json text"
    assert_output_contains '"seq":' "$output" "cat-frames json seq"

    # Test the spec's equals-sign argument form
    if output=$("$ASCIINEMA_BIN" cat-frames --frame=1,4-5 --no-escapes "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames equals-form frame"
    assert_output_contains '--- frame 4 @ 3.000000 (resize: 8x3) ---' "$output" "cat-frames equals-form header"

    # Test v1 input
    if output=$("$ASCIINEMA_BIN" cat-frames --frame 1 --no-escapes "$FIXTURES/minimal-v1.json" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames v1 input"

    # Test that a selection is required
    if output=$("$ASCIINEMA_BIN" cat-frames "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 2 "$rc" "cat-frames missing selection"

    # A selection that matches no frames warns instead of silently succeeding
    if output=$("$ASCIINEMA_BIN" cat-frames --frame 999 "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames unmatched selection"
    assert_output_contains 'no frames matched' "$output" "cat-frames unmatched selection warning"

    # --no-escapes is a no-op with --format json and says so
    if output=$("$ASCIINEMA_BIN" cat-frames --frame 1 --no-escapes --format json "$FIXTURES/frames-v3.cast" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames no-escapes json"
    assert_output_contains 'no effect with --format json' "$output" "cat-frames no-escapes json warning"

    # A malicious marker label must not inject escape sequences into the header
    local evil="$TMP_DATA_DIR/cat_frames_evil.cast"
    printf '%s\n' '{"version":3,"term":{"cols":20,"rows":3}}' '[0.1,"o","hi"]' '[0.2,"m","\u001b]0;x\u0007evil"]' > "$evil"
    if output=$("$ASCIINEMA_BIN" cat-frames --frame 2 --no-escapes "$evil" 2>&1); then rc=0; else rc=$?; fi
    assert_exit_code 0 "$rc" "cat-frames escaped marker"
    assert_output_contains 'marker: \u{1b}]0;x' "$output" "cat-frames marker label escaped"
}

# MAIN EXECUTION

# Setup always runs
setup

echo
echo "######################################################"
echo "# ASCIINEMA CLI INTEGRATION TESTS"
echo "######################################################"
echo "# Test filter: ${TEST:-ALL}"
echo "######################################################"

# Individual test blocks
run_test "help" test_help
run_test "version" test_version
run_test "auth" test_auth
run_test "record" test_record
run_test "play" test_play
run_test "stream" test_stream
run_test "session" test_session
run_test "cat" test_cat
run_test "convert" test_convert
run_test "ls-frames" test_ls_frames
run_test "cat-frames" test_cat_frames

# Final summary
echo
echo "######################################################"
echo "# TEST SUMMARY"
echo "######################################################"
echo "Tests run: $TESTS_RUN"
printf "%bTests passed: %b%s%b\n" "" "${GREEN}" "$TESTS_PASSED" "${NC}"
if [[ $TESTS_FAILED -gt 0 ]]; then
    printf "%bTests failed: %b%s%b\n" "" "${RED}" "$TESTS_FAILED" "${NC}"
    echo "OVERALL RESULT: FAILED"
    exit 1
else
    printf "%bTests failed: %b%s%b\n" "" "${GREEN}" "0" "${NC}"
    echo "OVERALL RESULT: SUCCESS"
fi
