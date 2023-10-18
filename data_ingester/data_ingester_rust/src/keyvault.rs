use std::sync::Arc;

use anyhow::Result;

use azure_identity::DefaultAzureCredential;
use azure_security_keyvault::KeyvaultClient;
use futures::try_join;

#[derive(Debug)]
pub(crate) struct Secrets {
    pub(crate) splunk_host: String,
    pub(crate) splunk_token: String,
    pub(crate) azure_client_id: String,
    pub(crate) azure_client_secret: String,
    pub(crate) azure_client_certificate: String,
    pub(crate) azure_client_organization: String,
    pub(crate) azure_tenant_id: String,
}

pub async fn get_keyvault_secrets(keyvault_name: &str) -> Result<Secrets> {
    let keyvault_url = format!("https://{keyvault_name}.vault.azure.net");
    let credential = Arc::new(DefaultAzureCredential::default());
    eprintln!("KeyVault Secret Client created");
    let secret_client = KeyvaultClient::new(&keyvault_url, credential.clone())?.secret_client();

    let splunk_host = secret_client.get("splunk-host").into_future();
    let splunk_token = secret_client.get("splunk-token").into_future();
    let azure_client_id = secret_client.get("ad-client-id").into_future();
    let azure_client_secret = secret_client.get("ad-client-secret").into_future();
    // Secret is automatically created when generating a certificate in KeyVault
    let azure_client_certificate = secret_client.get("ad-client-certificate").into_future();
    let azure_client_organization = secret_client.get("ad-client-organization").into_future();
    let azure_tenant_id = secret_client.get("ad-tenant-id").into_future();

    let (
        splunk_host,
        splunk_token,
        azure_client_id,
        azure_client_secret,
        azure_client_certificate,
        azure_client_organization,
        azure_tenant_id,
    ) = try_join!(
        splunk_host,
        splunk_token,
        azure_client_id,
        azure_client_secret,
        azure_client_certificate,
        azure_client_organization,
        azure_tenant_id
    )?;

    Ok(Secrets {
        splunk_host: splunk_host.value,
        splunk_token: splunk_token.value,
        azure_client_id: azure_client_id.value,
        azure_client_secret: azure_client_secret.value,
        azure_client_certificate: azure_client_certificate.value,
        azure_client_organization: azure_client_organization.value,
        azure_tenant_id: azure_tenant_id.value,
    })
}
