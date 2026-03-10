use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "geist-cli", about = "CLI client for geist-edge")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check if an agent operation is allowed by the edge pipeline
    Check {
        /// Agent identifier
        #[arg(long)]
        agent_id: String,

        /// Tool name
        #[arg(long)]
        tool: String,

        /// Resource path
        #[arg(long, default_value = "/")]
        resource: String,

        /// Operation type
        #[arg(long, default_value = "")]
        operation: String,

        /// Session identifier
        #[arg(long, default_value = "")]
        session_id: String,

        /// Server URL
        #[arg(long, default_value = "http://localhost:3000")]
        server: String,

        /// Request path (appended to server URL)
        #[arg(long, default_value = "/")]
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Check {
            agent_id,
            tool,
            resource,
            operation,
            session_id,
            server,
            path,
        } => check(server, path, agent_id, tool, resource, operation, session_id),
    }
}

fn check(
    server: String,
    path: String,
    agent_id: String,
    tool: String,
    resource: String,
    operation: String,
    session_id: String,
) {
    let url = format!("{}{}", server.trim_end_matches('/'), path);

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("x-geist-agent-id", &agent_id)
        .header("x-geist-tool-name", &tool)
        .header("x-geist-resource", &resource)
        .header("x-geist-operation", &operation)
        .header("x-geist-session-id", &session_id);

    match resp.send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response.text().unwrap_or_default();

            if status == 200 {
                println!("\x1b[32mALLOW\x1b[0m ({}): {}", status, body.trim());
            } else if status == 403 {
                println!("\x1b[31mDENY\x1b[0m ({}): {}", status, body.trim());
            } else {
                println!("\x1b[33mERROR\x1b[0m ({}): {}", status, body.trim());
            }
        }
        Err(e) => {
            eprintln!("\x1b[31mERROR\x1b[0m: {}", e);
            std::process::exit(1);
        }
    }
}
