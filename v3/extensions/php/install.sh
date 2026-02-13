#!/bin/bash
set -euo pipefail

# php install script - Simplified for YAML-driven architecture
# Installs PHP 8.4 + Composer + Symfony CLI + dev tools

print_status "Installing PHP and Symfony development environment..."

# Add ondrej/php PPA manually (avoids software-properties-common which pulls in python3)
if ! grep -q "ondrej/php" /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources 2>/dev/null; then
  print_status "Adding PHP repository..."
  UBUNTU_CODENAME=$(grep UBUNTU_CODENAME /etc/os-release | cut -d= -f2)
  sudo mkdir -p /etc/apt/keyrings
  curl -fsSL "https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x14AA40EC0831756756D7F66C4F4EA0AAE5267A6C" \
    | sudo gpg --dearmor -o /etc/apt/keyrings/ondrej-php.gpg
  echo "deb [signed-by=/etc/apt/keyrings/ondrej-php.gpg] https://ppa.launchpadcontent.net/ondrej/php/ubuntu ${UBUNTU_CODENAME} main" \
    | sudo tee /etc/apt/sources.list.d/ondrej-php.list > /dev/null
fi

# Update package lists
sudo apt-get update || exit 1

# Install PHP and extensions
print_status "Installing PHP 8.4 and extensions..."
php_packages=(
  php8.4 php8.4-cli php8.4-common php8.4-curl php8.4-mbstring
  php8.4-mysql php8.4-pgsql php8.4-sqlite3 php8.4-xml php8.4-zip
  php8.4-bcmath php8.4-gd php8.4-intl php8.4-opcache php8.4-readline
  php8.4-soap php8.4-xdebug php8.4-redis php8.4-amqp
  php8.4-mongodb php8.4-imagick
)

failed_packages=()
for package in "${php_packages[@]}"; do
  sudo apt-get install -y "$package" 2>/dev/null || failed_packages+=("$package")
done

# Check core PHP installed
if [[ " ${failed_packages[*]} " =~ php8.4 ]] || [[ " ${failed_packages[*]} " =~ php8.4-cli ]]; then
  print_error "Failed to install core PHP packages"
  exit 1
fi

[[ ${#failed_packages[@]} -gt 0 ]] && print_warning "Some optional extensions failed: ${failed_packages[*]}"

# Install development PHP ini (requires root — done here instead of configure phase)
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
if [[ -f "$SCRIPT_DIR/development-ini.template" ]] && [[ -d /etc/php/8.4/cli/conf.d ]]; then
  sudo cp "$SCRIPT_DIR/development-ini.template" /etc/php/8.4/cli/conf.d/99-development.ini
  print_success "PHP development ini installed"
fi

# Install Composer
if command_exists composer; then
  print_warning "Composer already installed"
else
  print_status "Installing Composer..."
  COMPOSER_TMP="/tmp/composer-install-$$"
  mkdir -p "$COMPOSER_TMP" && cd "$COMPOSER_TMP" || exit 1

  EXPECTED_CHECKSUM=$(timeout 30 php -r 'copy("https://composer.github.io/installer.sig", "php://stdout");' 2>/dev/null)
  [[ -z "$EXPECTED_CHECKSUM" ]] && print_error "Failed to fetch Composer signature" && exit 1

  curl --max-time 60 -fsSL https://getcomposer.org/installer -o composer-setup.php || exit 1
  ACTUAL_CHECKSUM=$(php -r "echo hash_file('sha384', 'composer-setup.php');")

  if [[ "$EXPECTED_CHECKSUM" != "$ACTUAL_CHECKSUM" ]]; then
    print_error "Composer checksum verification failed"
    cd - && rm -rf "$COMPOSER_TMP"
    exit 1
  fi

  php composer-setup.php && sudo mv composer.phar /usr/local/bin/composer && sudo chmod +x /usr/local/bin/composer
  cd - && rm -rf "$COMPOSER_TMP"
  print_success "Composer installed: $(composer --version 2>&1 | head -n1)"
fi

# Install Symfony CLI
if command_exists symfony; then
  print_warning "Symfony CLI already installed"
else
  print_status "Installing Symfony CLI..."
  if timeout 120 bash -c 'wget https://get.symfony.com/cli/installer -O - | bash' 2>/dev/null; then
    sudo mv "$HOME"/.symfony*/bin/symfony /usr/local/bin/symfony 2>/dev/null
    print_success "Symfony CLI installed"
  else
    print_warning "Symfony CLI installation failed"
  fi
fi

# Install PHP development tools via Composer
print_status "Installing PHP development tools..."
mkdir -p ~/.composer

php_tools=(
  "friendsofphp/php-cs-fixer"
  "phpstan/phpstan"
  "vimeo/psalm"
  "phpunit/phpunit"
  "squizlabs/php_codesniffer"
  "phpmd/phpmd"
  "psy/psysh"
)

for tool in "${php_tools[@]}"; do
  tool_binary=$(echo "$tool" | sed 's/.*\///')
  if command_exists "$tool_binary" || [[ -f "$HOME/.composer/vendor/bin/$tool_binary" ]]; then
    continue
  fi
  timeout 300 composer global require "$tool" 2>/dev/null || print_warning "Failed to install $tool"
done

print_success "PHP development environment installation complete"
