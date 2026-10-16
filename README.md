# Rusticity - Terminal UI for AWS

[![Crate Badge]][Crate] [![License Badge]][License] [![CI Badge]][CI] [![Built With Ratatui]][Ratatui]

[Crate]: https://crates.io/crates/rusticity
[License]: ./LICENSE
[CI]: https://github.com/clumsy/rusticity/actions
[Ratatui]: https://ratatui.rs/

[Crate Badge]: https://img.shields.io/crates/v/rusticity?logo=rust&style=flat-square&color=E05D44
[License Badge]: https://img.shields.io/crates/l/rusticity?style=flat-square&color=1370D3
[CI Badge]: https://img.shields.io/github/actions/workflow/status/clumsy/rusticity/ci.yml?style=flat-square&logo=github
[Built With Ratatui]: https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff&style=flat-square

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

- 🌐 **API Gateway**: REST APIs, HTTP APIs, and WebSocket APIs
- 📊 **CloudWatch Logs**: Log groups, log streams, log events
- 🔍 **CloudWatch Logs Insights**: Query and analyze logs
- 🚨 **CloudWatch Alarms**: View alarms with metrics graphs
- 🛤️ **CloudTrail**: Event history with resource details
- ☁️ **CloudFormation**: Stacks, resources, outputs, parameters, templates
- 🖥️ **EC2**: Instances with tags and monitoring
- 🗄️ **DynamoDB**: Tables with an item explorer (scan/query), indexes, global tables, backups, monitoring, settings and tags
- 📦 **ECR**: Elastic Container Registry (public and private repositories)
- 📂 **EFS**: Elastic File System — file systems with detail view and tags
- 🗄️ **FSx**: File systems (Windows, Lustre, ONTAP, OpenZFS) — list with customizable columns
- 👤 **IAM**: Users, roles, groups, policies, and permissions
- 🔑 **KMS**: Managed Keys — AWS managed and customer managed keys
- λ **Lambda**: Functions, versions, aliases, layers, applications, and monitoring
- 🛢️ **RDS**: Databases — read-only DB instance inventory with customizable metadata, maintenance, recommendations and monitoring columns
- 🪣 **S3**: Buckets and objects with navigation
- 📣 **SNS**: Topics
- 📬 **SQS**: Queues with monitoring and Lambda triggers
- 🌐 **VPC**: Read-only VPC inventory with IPv4/IPv6 CIDRs, default status, tenancy, owner and customizable columns

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

## Column Customization

Customize column names by adding a `[columns]` section to `~/.config/rusticity/config.toml`:

```toml
[columns.lambda.function]
name = "Function Name"
description = "Description"

[columns.cw.group]
log_group = "Log Group"
stored_bytes = "Storage"
```

## Usage

```bash
# Run the application
rusticity

# With specific AWS profile and region
AWS_PROFILE=your-profile AWS_DEFAULT_REGION=your-region rusticity
```

Use the status bar for shortcuts available in the current view, and press `?`
for keyboard help. Press `Space`, then `f` to find resources across services,
or `Ctrl+L` on a resource to explore related resources where supported.
Press `Ctrl+Y` on a resource or focused finder result to copy a read-only AWS CLI
command for your active profile and Region; it is never executed automatically.
Commands are quoted for POSIX shells (sh/bash/zsh). They contain no credentials;
the selected profile must be available wherever you paste them.
Press `Ctrl+E` from any service view or focused finder result to open a separate
**CLI output** tab for the supported read-only command. The command is shown at
the top; it describes the selected resource rather than reconstructing all tab
data. `[`/`]` switches tabs, arrows scroll, `Ctrl+U/Ctrl+D` page, `y` copies output,
`Ctrl+Y` copies the command, `Ctrl+R` reruns it, and `Ctrl+W`/Esc closes/cancels.
Reads can finish in the background while you use other tabs. AWS CLI v2 must be on
PATH with credentials for the active profile. Execution is limited to 30 seconds
and 1 MiB of stdout, uses no shell, and shows unsupported targets or failed reads
explicitly. Output tabs are not saved in sessions; their in-memory results may
contain sensitive AWS metadata and are discarded on close or AWS context change.

## Development

The project uses a Cargo workspace with three crates:

```
rusticity/
├── rusticity-core/    # Core AWS SDK integration
├── rusticity-term/    # Terminal UI components
└── rusticity/         # Main binary
```

### Setup

Requires **Rust 1.94.1 or newer** (the AWS SDK minimum). External dependency versions
are centralized in the workspace `Cargo.toml`; `Cargo.lock` is tracked for reproducible builds.

```bash
# Clone and build using the checked-in dependency resolution
cargo build --locked

# Run tests (also installs git hooks automatically)
cargo test --workspace --locked
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
