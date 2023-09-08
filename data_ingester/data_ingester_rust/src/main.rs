mod azure_functions;
mod conditional_access_policies;
mod directory_roles;
mod groups;
mod keyvault;
mod ms_graph;
mod roles;
mod splunk;
mod users;

use azure_functions::start_server;

#[tokio::main]
async fn main() {
    start_server().await
}

