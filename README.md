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

### Column Customization

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

### Resource finder

Press `Space`, then `f` to search supported AWS resource inventories in the current
profile and Region. Type at least three search characters (excluding qualifiers),
optionally adding `@s3`, `@dynamodb`/`@ddb`, `@ec2`, `@vpc`, `@rds`, `@iam`, `@lambda`, or
another supported service. The filter starts focused. Enter moves to the results;
there, use arrows to select and Enter to open. Tab/Shift+Tab switches focus,
`i` returns to the filter, Esc cancels, and `Ctrl+R` refreshes.

Search starts after a 250 ms pause and streams results without blocking typing.
It uses an existing AWS Resource Explorer view for indexed prefix search; it does
not create indexes or views. If unavailable, it searches supported inventories
with at most four concurrent sources and a 60-second in-memory cache. Cached
inventory matching is case-insensitive substring matching. Global services are
included where the underlying API is global. Changing profile/Region clears the
cache. Permission failures and result limits are shown rather than reported as
complete empty results; indexed results are limited to 1,000 and can lag AWS changes.

The finder searches top-level resources, not DynamoDB items, S3 objects, log
streams/events, or CloudTrail event contents. DynamoDB results open Items and S3
results open the bucket; other results open their filtered service inventory.

### DynamoDB Items

Press `m` in Normal mode to load the next Scan/Query page. Each request evaluates
at most 100 items and is also subject to DynamoDB's response-size limit. Page
navigation and Preferences only affect already-loaded items; they do not fetch
more data. `Ctrl+R` restarts the scan/query, or retries the failed page when the
error dialog is open. Changing the table, account context, index, projection,
operation, or query key starts a new result set. Pagination shows only loaded
pages followed by `...` while the total is unknown; the ellipsis disappears
only after the final result page has loaded successfully.

### RDS Databases

Preferences lists database metadata plus optional monitoring, maintenance, and
recommendation columns. Extra read APIs are called only when their columns are
visible. Missing or inapplicable values show `-`; failed optional reads show
`Unavailable` without hiding the inventory. CPU, current activity (average active
sessions), and replica lag use the latest CloudWatch sample within 15 minutes;
metrics not published for an instance remain `-`.

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
