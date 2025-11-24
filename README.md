# Rusticity - Terminal UI for AWS 

A snappy terminal UI for AWS written in 🦀 Rust, inspired by 🧬 Helix editor.

> ⚠️ **Early Development**: Rusticity is in active development. Expect breaking changes, bugs, and evolving features. Use at your own risk and please report any issues you encounter!

## Features

- 📟 Terminal UI using Ratatui
- ⌨️ Helix-like keybindings with Normal and Insert modes
- 🪟 Multi-pane support
- 🏗️ Modular architecture with separate crates:
  - `rusticity-core`: AWS SDK integration
  - `rusticity-term`: Terminal UI components
  - `rusticity`: Main binary

## Supported Services

- 📊 **CloudWatch Logs**: Log groups, log streams, log events
- 🔍 **CloudWatch Logs Insights**: Query and analyze logs
- 🚨 **CloudWatch Alarms**: View and manage alarms
- 🪣 **S3**: Browse buckets and objects
- 📦 **ECR**: Elastic Container Registry (public and private)
- ☁️ **CloudFormation**: View and manage stacks
- λ **Lambda**: Function management
- 👤 **IAM**: Identity and Access Management

**Want to see support for another AWS service?** [Request it here](../../issues/new?template=feature_request.md) or upvote existing requests!

## Architecture

Inspired by Helix editor (https://github.com/helix-editor/helix), the project is split into:

- **rusticity-core**: Core AWS functionality (SDK clients, API calls, data models)
- **rusticity-term**: Terminal UI layer (event handling, keymaps, rendering, pane management)
- **rusticity**: Main application binary

## Installation

### From crates.io

```bash
cargo install rusticity
```

### From source

```bash
git clone https://github.com/clumsy/rusticity.git
cd rusticity
cargo install --path rusticity
```

## Configuration

Configure AWS credentials using standard AWS methods:

```bash
# Using environment variables
export AWS_DEFAULT_REGION=your-region
export AWS_PROFILE=your-profile
export AWS_ACCESS_KEY_ID=your-key
export AWS_SECRET_ACCESS_KEY=your-secret

# Or use AWS CLI configuration
aws configure
```

### i18n and Customization

Customize column names by creating `~/.config/rusticity/i18n.toml`:

```toml
[column.lambda.function]
name = "Function Name"
description = "Description"
```

## Usage

```bash
# Run the application
rusticity

# With specific AWS profile and region
AWS_PROFILE=your-profile AWS_DEFAULT_REGION=your-region rusticity
```

## Development

The project uses a Cargo workspace with three crates:

```
rusticity/
├── rusticity-core/    # Core AWS SDK integration
├── rusticity-term/    # Terminal UI components
└── rusticity/         # Main binary
```

### Setup

```bash
# Clone and build
cargo build

# Run tests (also installs git hooks automatically)
cargo test
```

Git hooks will automatically check formatting and linting before commits.

### Run

```bash
# Run the application
cargo run

# With specific AWS profile and region
AWS_PROFILE=your-profile AWS_DEFAULT_REGION=your-region cargo run
```

### Testing

`cargo test` runs all checks including formatting, linting, and tests:

```bash
cargo test  # Runs fmt check, clippy, and all tests
```

## License

Apache-2.0
