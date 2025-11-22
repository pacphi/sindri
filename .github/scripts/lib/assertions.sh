#!/bin/bash
# Assertion functions for testing

# Assert command exists
assert_command() {
    local command="$1"
    local app_name="${2:-}"

    if [[ -n "$app_name" ]]; then
        # Remote assertion
        if flyctl ssh console -a "$app_name" --command "command -v $command" &>/dev/null; then
            echo "✓ Command '$command' exists"
            return 0
        else
            echo "✗ Command '$command' not found"
            return 1
        fi
    else
        # Local assertion
        if command -v "$command" &>/dev/null; then
            echo "✓ Command '$command' exists"
            return 0
        else
            echo "✗ Command '$command' not found"
            return 1
        fi
    fi
}

# Assert file exists
assert_file_exists() {
    local file="$1"
    local app_name="${2:-}"

    if [[ -n "$app_name" ]]; then
        if flyctl ssh console -a "$app_name" --command "test -f $file"; then
            echo "✓ File '$file' exists"
            return 0
        else
            echo "✗ File '$file' not found"
            return 1
        fi
    else
        if [[ -f "$file" ]]; then
            echo "✓ File '$file' exists"
            return 0
        else
            echo "✗ File '$file' not found"
            return 1
        fi
    fi
}

# Assert directory exists
assert_directory_exists() {
    local dir="$1"
    local app_name="${2:-}"

    if [[ -n "$app_name" ]]; then
        if flyctl ssh console -a "$app_name" --command "test -d $dir"; then
            echo "✓ Directory '$dir' exists"
            return 0
        else
            echo "✗ Directory '$dir' not found"
            return 1
        fi
    else
        if [[ -d "$dir" ]]; then
            echo "✓ Directory '$dir' exists"
            return 0
        else
            echo "✗ Directory '$dir' not found"
            return 1
        fi
    fi
}

# Assert strings are equal
assert_equals() {
    local expected="$1"
    local actual="$2"
    local message="${3:-Values should be equal}"

    if [[ "$expected" == "$actual" ]]; then
        echo "✓ $message"
        return 0
    else
        echo "✗ $message"
        echo "  Expected: '$expected'"
        echo "  Actual:   '$actual'"
        return 1
    fi
}

# Assert string contains substring
assert_contains() {
    local haystack="$1"
    local needle="$2"
    local message="${3:-String should contain substring}"

    if [[ "$haystack" == *"$needle"* ]]; then
        echo "✓ $message"
        return 0
    else
        echo "✗ $message"
        echo "  String: '$haystack'"
        echo "  Should contain: '$needle'"
        return 1
    fi
}

# Assert command succeeds
assert_success() {
    local command="$1"
    local message="${2:-Command should succeed}"

    if eval "$command" &>/dev/null; then
        echo "✓ $message"
        return 0
    else
        echo "✗ $message"
        echo "  Command: $command"
        return 1
    fi
}

# Assert command fails
assert_failure() {
    local command="$1"
    local message="${2:-Command should fail}"

    if eval "$command" &>/dev/null; then
        echo "✗ $message"
        echo "  Command: $command"
        return 1
    else
        echo "✓ $message"
        return 0
    fi
}

# Assert exit code
assert_exit_code() {
    local expected="$1"
    local command="$2"
    local message="${3:-Exit code should match}"

    eval "$command" &>/dev/null
    local actual=$?

    if [[ $actual -eq $expected ]]; then
        echo "✓ $message (exit code: $expected)"
        return 0
    else
        echo "✗ $message"
        echo "  Expected exit code: $expected"
        echo "  Actual exit code: $actual"
        return 1
    fi
}

# Assert output matches regex
assert_matches() {
    local pattern="$1"
    local text="$2"
    local message="${3:-Text should match pattern}"

    if [[ "$text" =~ $pattern ]]; then
        echo "✓ $message"
        return 0
    else
        echo "✗ $message"
        echo "  Pattern: $pattern"
        echo "  Text: $text"
        return 1
    fi
}

# Assert numeric comparison
assert_numeric() {
    local operator="$1"  # -eq, -ne, -lt, -le, -gt, -ge
    local value1="$2"
    local value2="$3"
    local message="${4:-Numeric assertion}"

    if test "$value1" "$operator" "$value2"; then
        echo "✓ $message ($value1 $operator $value2)"
        return 0
    else
        echo "✗ $message"
        echo "  Assertion: $value1 $operator $value2"
        return 1
    fi
}