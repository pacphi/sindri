# Ruby Extension

> Version: 2.0.0 | Category: languages | Last Updated: 2026-01-26

## Overview

Ruby 3.4.7 via mise with Rails and Bundler. Provides a complete Ruby development environment with modern tooling.

## What It Provides

| Tool   | Type            | License              | Description                |
| ------ | --------------- | -------------------- | -------------------------- |
| ruby   | runtime         | Ruby OR BSD-2-Clause | Ruby interpreter           |
| gem    | package-manager | Ruby OR MIT          | RubyGems package manager   |
| bundle | cli-tool        | MIT                  | Bundler dependency manager |

## Requirements

- **Disk Space**: 2500 MB
- **Memory**: 512 MB
- **Install Time**: ~90 seconds
- **Dependencies**: None

### Network Domains

- rubygems.org
- github.com

## Installation

```bash
extension-manager install ruby
```

## Configuration

### Templates

| Template                  | Destination              | Description            |
| ------------------------- | ------------------------ | ---------------------- |
| bashrc-aliases.template   | ~/.bashrc                | Shell aliases for Ruby |
| gemfile-template.template | ~/templates/Gemfile      | Template Gemfile       |
| rubocop-config.template   | ~/templates/.rubocop.yml | RuboCop configuration  |

### Install Method

Uses a custom installation script with 900 second timeout.

### Upgrade Strategy

Automatic via mise upgrade.

## Usage Examples

### Basic Ruby

```bash
# Check version
ruby --version

# Run a script
ruby script.rb

# Interactive console
irb

# Evaluate code
ruby -e 'puts "Hello, Ruby!"'
```

### RubyGems

```bash
# Install a gem
gem install rails

# List installed gems
gem list

# Update gems
gem update

# Uninstall a gem
gem uninstall rails
```

### Bundler

```bash
# Initialize Bundler in a project
bundle init

# Install dependencies
bundle install

# Add a gem to Gemfile
bundle add rails

# Update dependencies
bundle update

# Execute command with bundle context
bundle exec rake test
```

### Rails (if installed)

```bash
# Create a new Rails app
rails new myapp

# Generate scaffold
rails generate scaffold Post title:string body:text

# Run migrations
rails db:migrate

# Start server
rails server

# Rails console
rails console
```

### RuboCop

```bash
# Check code style
rubocop

# Auto-correct issues
rubocop -a

# Auto-correct with unsafe corrections
rubocop -A
```

### Testing with RSpec

```bash
# Initialize RSpec
rspec --init

# Run tests
rspec

# Run specific test
rspec spec/models/user_spec.rb
```

## Validation

The extension validates the following commands:

- `ruby` - Must match pattern `ruby \d+\.\d+\.\d+`
- `gem` - Must be available
- `bundle` - Must be available

## Removal

```bash
extension-manager remove ruby
```

This removes:

- mise Ruby tools
- ~/templates/Gemfile
- ~/templates/.rubocop.yml

## Related Extensions

None - Ruby is a standalone language extension.
