# Create a jellyfin compatible server by porting jellofin-server to Rust

In the subdirectory jellofin-server, you will find an implementation of a jellyfin-compatible
server written in Go. This needs to be ported to Rust.

You are allowed to run 'cargo' commands as needed.

## Techstack

* List of cargo crates

- webserver: axum
- database: sqlx (using the sqlite backend)
- executor: tokio
- image processing: image
- index/search: tantivity
- command line options: clap (using clap::Parser)

* lib / bin separation

- The workspace has been set up in library mode. All the functonality will be
  implemented in the library. The server is just a file in bin/main.rs that processes
  the command line options and calls the main functionality from the library

Phase 1: study

Read the README in jellofin-server/ , then examine the Go files. Make a global plan
to port the server to Rust. Present this plan to the user.

Phase 2: create a detailed project plan

Create a detailed project plan that outlines the steps needed to port the server to Rust. Save it in project-plan.md
Present this plan to the user.

Phase 3: implementation

Port the code from Go to Rust.

