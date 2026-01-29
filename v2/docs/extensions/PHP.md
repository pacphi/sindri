# PHP

PHP 8.4 with Composer, Symfony CLI, and development tools.

## Overview

| Property         | Value    |
| ---------------- | -------- |
| **Category**     | language |
| **Version**      | 2.1.0    |
| **Installation** | script   |
| **Disk Space**   | 1000 MB  |
| **Dependencies** | None     |

## Description

PHP 8.4 with Composer, Symfony CLI, and development tools - provides a complete PHP development environment for modern web applications.

## Installed Tools

| Tool       | Type            | Description            |
| ---------- | --------------- | ---------------------- |
| `php`      | runtime         | PHP 8.4 interpreter    |
| `composer` | package-manager | PHP dependency manager |
| `symfony`  | cli-tool        | Symfony CLI            |

## Configuration

### Templates

| Template                   | Destination                                  | Description            |
| -------------------------- | -------------------------------------------- | ---------------------- |
| `bashrc-aliases.template`  | `~/.bashrc`                                  | PHP aliases            |
| `ssh-environment.template` | `/etc/profile.d/00-ssh-environment.sh`       | SSH environment        |
| `development-ini.template` | `/etc/php/8.4/cli/conf.d/99-development.ini` | Development PHP config |
| `cs-fixer-config.template` | `/workspace/templates/.php-cs-fixer.php`     | PHP CS Fixer config    |

### Development INI

```ini
display_errors = On
error_reporting = E_ALL
```

## Network Requirements

- `composer.github.io` - Composer
- `getcomposer.org` - Composer downloads
- `ppa.launchpadcontent.net` - PHP PPA
- `get.symfony.com` - Symfony CLI

## Installation

```bash
extension-manager install php
```

## Validation

```bash
php --version       # Expected: PHP 8.4.X
composer --version
symfony --version
```

## Upgrade

**Strategy:** automatic

Automatically updates PHP via apt.

## Removal

```bash
extension-manager remove php
```

Removes:

- `/usr/bin/composer`
- `/usr/local/bin/symfony`
- Template files
