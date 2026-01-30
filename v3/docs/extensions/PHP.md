# PHP Extension

> Version: 2.1.0 | Category: languages | Last Updated: 2026-01-26

## Overview

PHP 8.4 with Composer, Symfony CLI, and development tools. Provides a complete PHP development environment for modern web applications.

## What It Provides

| Tool     | Type            | License  | Description                           |
| -------- | --------------- | -------- | ------------------------------------- |
| php      | runtime         | PHP-3.01 | PHP 8.4 interpreter                   |
| composer | package-manager | MIT      | PHP dependency manager                |
| symfony  | cli-tool        | MIT      | Symfony CLI for framework development |

## Requirements

- **Disk Space**: 300 MB
- **Memory**: 256 MB
- **Install Time**: ~60 seconds
- **Dependencies**: None

### Network Domains

- github.io
- composer.github.io
- getcomposer.org
- ppa.launchpadcontent.net
- symfony.com
- get.symfony.com

## Installation

```bash
sindri extension install php
```

## Configuration

### Templates

| Template                 | Destination                                | Description                |
| ------------------------ | ------------------------------------------ | -------------------------- |
| bashrc-aliases.template  | ~/.bashrc                                  | Shell aliases for PHP      |
| ssh-environment.template | ~/.profile                                 | SSH environment setup      |
| development-ini.template | /etc/php/8.4/cli/conf.d/99-development.ini | PHP development settings   |
| cs-fixer-config.template | ~/templates/.php-cs-fixer.php              | PHP CS Fixer configuration |

### Install Method

Uses a custom installation script with 900 second timeout.

### Upgrade Strategy

Automatic via apt packages.

## Usage Examples

### Basic PHP

```bash
# Check version
php --version

# Run a script
php script.php

# Built-in development server
php -S localhost:8000

# Interactive shell
php -a

# Check configuration
php --ini
```

### Composer

```bash
# Create a new project
composer create-project symfony/skeleton my-project

# Install dependencies
composer install

# Add a package
composer require monolog/monolog

# Add a dev dependency
composer require --dev phpunit/phpunit

# Update dependencies
composer update

# Autoload dump
composer dump-autoload
```

### Symfony CLI

```bash
# Create a Symfony project
symfony new my_project --webapp

# Start local server
symfony serve

# Check requirements
symfony check:requirements

# Run console commands
symfony console make:controller

# Deploy
symfony cloud:deploy
```

### Laravel (via Composer)

```bash
# Create Laravel project
composer create-project laravel/laravel my-project

# Serve application
php artisan serve

# Run migrations
php artisan migrate

# Generate code
php artisan make:model Post -mcf
```

### PHPUnit Testing

```bash
# Run tests
./vendor/bin/phpunit

# Run with coverage
./vendor/bin/phpunit --coverage-html coverage
```

### PHP CS Fixer

```bash
# Check code style
./vendor/bin/php-cs-fixer fix --dry-run --diff

# Fix code style
./vendor/bin/php-cs-fixer fix
```

### Static Analysis

```bash
# PHPStan
./vendor/bin/phpstan analyse src

# Psalm
./vendor/bin/psalm
```

## Validation

The extension validates the following commands:

- `php` - Must match pattern `PHP 8\.4`
- `composer` - Must be available
- `symfony` - Must be available

## Removal

```bash
sindri extension remove php
```

This removes:

- /usr/bin/composer
- /usr/local/bin/symfony
- ~/templates/.php-cs-fixer.php

## Related Extensions

None - PHP is a standalone language extension.
