mod admin_request_consent_policy;
mod aws;
mod aws_alternate_contact_information;
mod aws_config;
mod aws_ec2;
mod aws_entities_for_policy;
mod aws_iam;
mod aws_kms;
mod aws_policy;
mod aws_s3;
mod aws_s3control;
mod aws_securityhub;
mod aws_trail;
mod azure_functions;
mod azure_rest;
mod conditional_access_policies;
mod directory_roles;
mod dns;
mod groups;
mod keyvault;
mod ms_graph;
mod msgraph_data;
mod powershell;
mod roles;
mod security_score;
mod splunk;
mod users;
use anyhow::Result;
use azure_functions::start_server;
use memory_stats::memory_stats;
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("Starting Data Ingester...");
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(start_server(tx));
    let _ = rx.await;
    eprintln!("Warp server started...");
    if let Some(usage) = memory_stats() {
        println!(
            "Current physical memory usage: {}",
            usage.physical_mem / 1_000_000
        );
        println!(
            "Current virtual memory usage: {}",
            usage.virtual_mem / 1_000_000
        );
    } else {
        println!("Couldn't get the current memory usage :(");
    }
    server.await??;
    if let Some(usage) = memory_stats() {
        println!(
            "Current physical memory usage: {}",
            usage.physical_mem / 1_000_000
        );
        println!(
            "Current virtual memory usage: {}",
            usage.virtual_mem / 1_000_000
        );
    } else {
        println!("Couldn't get the current memory usage :(");
    }
    Ok(())
}
