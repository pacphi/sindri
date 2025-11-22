# Project Management in Sindri

Sindri provides two powerful commands for project management: `clone-project` and `new-project`. These tools help you quickly set up new development projects or clone existing repositories with Claude AI enhancements.

## Quick Start

```bash
# Create a new Node.js project
./cli/new-project my-app --type node

# Clone and enhance an existing repository
./cli/clone-project https://github.com/user/repo

# Fork a repository and set up for contributions
./cli/clone-project https://github.com/upstream/repo --fork
```

## `new-project` - Create Projects from Templates

Create new projects with intelligent type detection, pre-configured templates, and automatic extension activation.

### new-project Usage

```bash
./cli/new-project <project_name> [options]
```

### new-project Options

- `--type <type>` - Specify project type explicitly (node, python, go, rust, rails, django, spring, dotnet, terraform, docker)
- `--list-types` - Show all available project types
- `--interactive` - Force interactive type selection
- `--git-name <name>` - Git user name for this project
- `--git-email <email>` - Git user email for this project
- `-h, --help` - Show help message

### new-project Examples

```bash
# Auto-detect from project name
./cli/new-project my-rails-app

# Explicitly specify type
./cli/new-project my-api --type python

# Interactive selection with custom Git config
./cli/new-project my-app --interactive --git-name "Jane Doe" --git-email "jane@example.com"

# List all available templates
./cli/new-project --list-types
```

### new-project How It Works

1. **Type Detection**: Automatically detects project type from the name
   - `my-rails-app` → Rails
   - `api-server` → Prompts for API type
   - `my-terraform-infra` → Terraform

2. **Template Application**: Applies project template including:
   - Initial directory structure
   - Template files (package.json, requirements.txt, etc.)
   - Project-specific .gitignore
   - CLAUDE.md context file

3. **Extension Activation**: Automatically installs required Sindri extensions:
   - Node.js projects get `nodejs` extension
   - Python projects get `python` extension
   - And so on...

4. **Git Initialization**: Sets up Git repository with:
   - Initial commit
   - Pre-commit hooks for code quality
   - Commit message validation
   - Custom Git configuration (if specified)

5. **Claude Tools**: Initializes Claude AI tools:
   - Claude Code project context
   - GitHub spec-kit (if uv available)
   - Claude Flow (if npx available)
   - .envrc for direnv integration

### Available Project Types

| Type        | Description               | Extensions  |
| ----------- | ------------------------- | ----------- |
| `node`      | Node.js application       | nodejs      |
| `python`    | Python application        | python      |
| `go`        | Go application            | golang      |
| `rust`      | Rust application          | rust        |
| `rails`     | Ruby on Rails application | ruby        |
| `django`    | Django web application    | python      |
| `spring`    | Spring Boot application   | jvm         |
| `dotnet`    | .NET application          | dotnet      |
| `terraform` | Terraform infrastructure  | infra-tools |
| `docker`    | Dockerized application    | docker      |

## `clone-project` - Clone with Enhancements

Clone or fork existing repositories and automatically apply Claude AI enhancements.

### clone-project Usage

```bash
./cli/clone-project <repository-url> [options]
```

### clone-project Options

- `--fork` - Fork repo before cloning (requires gh CLI)
- `--branch <name>` - Checkout specific branch after clone
- `--depth <n>` - Shallow clone with n commits
- `--git-name <name>` - Configure Git user name for this project
- `--git-email <email>` - Configure Git user email for this project
- `--feature <name>` - Create and checkout feature branch after clone
- `--no-deps` - Skip dependency installation
- `--no-enhance` - Skip all enhancements (just clone/fork)
- `-h, --help` - Show help message

### clone-project Examples

```bash
# Simple clone with enhancements
./cli/clone-project https://github.com/user/my-app

# Fork for contribution
./cli/clone-project https://github.com/original/project --fork

# Fork and create feature branch
./cli/clone-project https://github.com/original/project --fork --feature add-new-feature

# Clone with custom Git config
./cli/clone-project https://github.com/company/app --git-name "John Doe" --git-email "john@company.com"

# Shallow clone without enhancements
./cli/clone-project https://github.com/large/repo --depth 1 --no-enhance
```

### clone-project How It Works

1. **Fork/Clone**: Either clones directly or forks first (using GitHub CLI)

2. **Git Setup**: Configures Git for the project:
   - Sets up fork remotes (if forking)
   - Adds fork management aliases
   - Applies custom Git config (if specified)

3. **Enhancements Applied** (unless `--no-enhance`):
   - **Git Hooks**: Pre-commit and commit-msg hooks
   - **CLAUDE.md**: Creates project context file (via `claude /init` if available)
   - **Dependency Installation**: Automatically detects and installs:
     - npm install (for package.json)
     - pip3 install (for requirements.txt)
     - go mod download (for go.mod)
     - cargo build (for Cargo.toml)
     - bundle install (for Gemfile)
   - **Claude Tools**: Initializes spec-kit, Claude Flow, etc.

4. **Feature Branch** (if specified): Creates and checks out a feature branch

### Fork Management

When using `--fork`, the following Git aliases are automatically configured:

```bash
git sync-upstream         # Fetch and merge upstream changes
git push-fork            # Push current branch to your fork
git update-from-upstream # Rebase current branch on upstream/main
git pr-branch <name>     # Create new branch from upstream/main
git fork-status          # Show fork remotes and branch tracking
```

## Project Structure

Both commands create/enhance projects with the following structure:

```text
project-name/
├── .git/                 # Git repository
│   └── hooks/           # Pre-commit and commit-msg hooks
├── .gitignore           # Language-specific ignores
├── CLAUDE.md            # Claude Code project context
├── .envrc               # direnv configuration (if tools initialized)
├── [template files]     # Language/framework-specific files
└── [your code]
```

## Environment Variables

The scripts respect the following environment variables:

- `WORKSPACE_PROJECTS` - Base directory for projects (default: `$HOME/projects`)
- `DOCKER_LIB` - Location of Sindri library files
- `DEBUG` - Set to `true` for debug output

## Integration with Sindri Extensions

Projects automatically activate relevant extensions:

- **Language Extensions**: nodejs, python, golang, rust, ruby, jvm, dotnet
- **Infrastructure Extensions**: infra-tools, docker
- **AI Tools**: ai-toolkit (if needed)

These extensions are installed using the `extension-manager` and configured for the project.

## Best Practices

1. **Use Templates**: Let auto-detection work for you - name your projects descriptively
2. **Fork for Contributions**: Always use `--fork` when contributing to open source
3. **Feature Branches**: Use `--feature` to immediately start working on a specific task
4. **Custom Git Config**: Use project-specific Git configs for work vs. personal projects
5. **Skip Enhancements Wisely**: Only use `--no-enhance` for quick inspections

## Troubleshooting

### yq not found

```bash
# macOS
brew install yq

# Linux (Ubuntu/Debian)
sudo apt install yq
```

### GitHub CLI not authenticated (for --fork)

```bash
gh auth login
```

### Dependencies fail to install

- Check that the runtime is installed (node, python3, go, cargo, ruby)
- Use `--no-deps` to skip and install manually

### Permission errors

- Ensure the project directory is writable
- Check that WORKSPACE_PROJECTS directory exists

## Advanced Usage

### Custom Templates

Templates are defined in `docker/lib/project-templates.yaml`. You can add your own:

```yaml
templates:
  mytemplate:
    description: "My custom template"
    extensions:
      - nodejs
    setup_commands:
      - "npm init -y"
    files:
      "README.md": |
        # {project_name}
        Custom template
    claude_md_template: |
      # {project_name}

      Custom CLAUDE.md template
```

### Template Variables

Available variables in templates:

- `{project_name}` - Project name
- `{author}` - Git user name
- `{date}` - Current date (YYYY-MM-DD)
- `{year}` - Current year
- `{git_user_name}` - Git user name
- `{git_user_email}` - Git user email

## See Also

- [Extension Manager](../cli/extension-manager) - Manage Sindri extensions
- [CLAUDE.md](../CLAUDE.md) - Project documentation guidelines
- [Extension Development](./EXTENSION_DEVELOPMENT.md) - Create custom extensions
