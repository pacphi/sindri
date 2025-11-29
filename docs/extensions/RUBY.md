# Ruby

Ruby 3.4.7 with Rails and Bundler via mise.

## Overview

| Property         | Value    |
| ---------------- | -------- |
| **Category**     | language |
| **Version**      | 2.0.0    |
| **Installation** | script   |
| **Disk Space**   | 2500 MB  |
| **Dependencies** | None     |

## Description

Ruby 3.4.7 via mise with Rails and Bundler - provides a complete Ruby development environment for web applications.

## Installed Tools

| Tool     | Type            | Description                |
| -------- | --------------- | -------------------------- |
| `ruby`   | runtime         | Ruby 3.4.7 interpreter     |
| `gem`    | package-manager | RubyGems package manager   |
| `bundle` | cli-tool        | Bundler dependency manager |

## Configuration

### Templates

| Template                    | Destination                         | Description           |
| --------------------------- | ----------------------------------- | --------------------- |
| `bashrc-aliases.template`   | `~/.bashrc`                         | Ruby-specific aliases |
| `gemfile-template.template` | `/workspace/templates/Gemfile`      | Template Gemfile      |
| `rubocop-config.template`   | `/workspace/templates/.rubocop.yml` | Rubocop configuration |

## Network Requirements

- `rubygems.org` - RubyGems registry
- `github.com` - GitHub dependencies

## Installation

```bash
extension-manager install ruby
```

## Validation

```bash
ruby --version    # Expected: ruby X.X.X
gem --version
bundle --version
```

## Upgrade

**Strategy:** automatic

Automatically upgrades via mise.

## Removal

```bash
extension-manager remove ruby
```

Removes mise configuration, Ruby installation, and template files.
