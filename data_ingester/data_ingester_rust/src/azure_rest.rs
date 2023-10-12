use azure_mgmt_authorization::models::role_assignment_properties::PrincipalType;
use azure_mgmt_authorization::{
    models::{RoleAssignment as SDKRoleAssignment, RoleDefinition as SDKRoleDefinition},
    package_2022_04_01::Client as ClientAuthorization,
};
use azure_mgmt_subscription::{
    models::Subscription, package_2021_10::Client as ClientSubscription,
};

use anyhow::{Context, Result};
use azure_core::auth::TokenCredential;
use azure_identity::ClientSecretCredential;
use azure_identity::TokenCredentialOptions;
use dyn_fmt::AsStrFormatExt;
use futures::stream::StreamExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use url::Url;

use crate::splunk::{HecEvent, ToHecEvents};

pub struct AzureRest {
    credential: Arc<ClientSecretCredential>,
    subscriptions: Subscriptions,
}

impl AzureRest {
    pub async fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<Self> {
        //let credential = Arc::new(DefaultAzureCredential::default());
        let http_client = azure_core::new_http_client();
        let credential = Arc::new(ClientSecretCredential::new(
            http_client,
            tenant_id.to_owned(),
            client_id.to_owned(),
            client_secret.to_owned(),
            TokenCredentialOptions::default(),
        ));
        let mut s = Self {
            credential,
            subscriptions: Subscriptions { inner: vec![] },
        };
        s.subscriptions = s.azure_subscriptions().await?;
        Ok(s)
    }

    pub async fn azure_subscriptions(&self) -> Result<Subscriptions> {
        let client = ClientSubscription::builder(self.credential.clone()).build();
        let mut stream = client.subscriptions_client().list().into_stream();
        let mut subscriptions = vec![];
        while let Some(item) = stream.next().await {
            for sub in item?.value {
                subscriptions.push(sub);
            }
        }
        Ok(Subscriptions {
            inner: subscriptions,
        })
    }

    pub async fn get_security_contacts(&self) -> Result<Vec<HecEvent>> {
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Security/securityContacts?api-version=2020-01-01-preview";
        let results = self.rest_request_subscription_iter(url_template).await?;
        Ok(results)
    }

    pub async fn get_security_center_built_in(&self) -> Result<Vec<HecEvent>> {
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Authorization/policyAssignments/SecurityCenterBuiltIn?api-version=2021-06-01";
        let results = self.rest_request_subscription_iter(url_template).await?;
        Ok(results)
    }

    // pub async fn get_microsoft_authorization_role_definitions(&self) -> Result<Vec<HecEvent>> {
    //     let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Authorization/roleDefinitions?api-version=2017-05-01";
    //     let results = self.rest_request_subscription_iter(url_template).await?;
    //     Ok(results)
    // }

    pub async fn azure_role_definitions(&self) -> Result<HashMap<String, RoleDefinition>> {
        let client = ClientAuthorization::builder(self.credential.clone()).build();
        let mut collection = HashMap::new();
        for sub in self.subscriptions.inner.iter() {
            let sub_id = sub.subscription_id.as_ref().context("no sub id")?;
            let scope = format!("/subscriptions/{}", sub_id);
            let mut stream = client.role_definitions_client().list(scope).into_stream();
            while let Some(results) = stream.next().await {
                for item in results?.value {
                    collection.insert(
                        item.id
                            .as_ref()
                            .context("No ID on role definition")?
                            .to_owned(),
                        RoleDefinition(item),
                    );
                }
            }
        }
        Ok(collection)
    }

    pub async fn azure_role_assignments(&self) -> Result<HashMap<String, RoleAssignment>> {
        let client = ClientAuthorization::builder(self.credential.clone()).build();
        let mut collection = HashMap::new();
        for sub in self.subscriptions.inner.iter() {
            let sub_id = sub.subscription_id.as_ref().context("no sub id")?;
            let scope = format!("/subscriptions/{}", sub_id);
            let mut stream = client
                .role_assignments_client()
                .list_for_scope(scope)
                .into_stream();
            while let Some(results) = stream.next().await {
                for item in results?.value {
                    collection.insert(
                        item.id
                            .as_ref()
                            .context("No ID on role assignment")?
                            .to_owned(),
                        RoleAssignment(item),
                    );
                }
            }
        }
        Ok(collection)
    }

    // pub async fn get_microsoft_authorization_role_assignments(&self) -> Result<Vec<HecEvent>> {
    //     let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Authorization/roleassignments?api-version=2017-10-01-preview";
    //     let results = self.rest_request_subscription_iter(url_template).await?;
    //     Ok(results)
    // }

    pub async fn get_microsoft_security_settings(&self) -> Result<Vec<HecEvent>> {
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Security/settings?api-version=2021-06-01";
        let results = self.rest_request_subscription_iter(url_template).await?;
        Ok(results)
    }

    pub async fn get_microsoft_security_auto_provisioning_settings(&self) -> Result<Vec<HecEvent>> {
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Security/autoProvisioningSettings?api-version=2017-08-01-preview";
        let results = self.rest_request_subscription_iter(url_template).await?;
        Ok(results)
    }

    pub async fn get_microsoft_sql_encryption_protection(&self) -> Result<Vec<HecEvent>> {
        let mut collection: Vec<HecEvent> = vec![];
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Sql/servers?api-version=2022-05-01-preview";
        let results = self
            .rest_request_subscription_iter_no_hec(url_template)
            .await?;

        for entry in results.iter() {
            match entry {
                ReturnType::Collection {
                    value,
                    next_link: _,
                } => {
                    for server in value.iter() {
                        let url = format!(
                            "https://management.azure.com{}/encryptionProtector?api-version=2022-05-01-preview",
                            server.as_object().unwrap().get("id").unwrap().as_str().unwrap());
                        let result = self.rest_request::<ReturnType>(&url).await?;
                        collection.extend(result.to_hec_events(&url)?);
                    }
                }
                _ => unreachable!(),
            };
        }

        Ok(collection)
    }

    pub async fn rest_request<T: DeserializeOwned + std::fmt::Debug>(
        &self,
        url: &str,
    ) -> Result<T> {
        let response = self
            .credential
            .get_token("https://management.azure.com")
            .await?;

        let response = reqwest::Client::new()
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", response.token.secret()),
            )
            .send()
            .await?
            .text()
            .await?;

        let rt: T = serde_json::from_str(&response)?;
        dbg!(&rt);
        Ok(rt)
    }

    pub async fn rest_request_subscription_iter_no_hec(
        &self,
        url_template: &str,
    ) -> Result<Vec<ReturnType>> {
        let response = self
            .credential
            .get_token("https://management.azure.com")
            .await?;

        let mut collection = vec![];
        for sub in self.subscriptions.inner.iter() {
            let sub_id = sub.subscription_id.as_ref().context("no sub id")?;
            let url = Url::parse(&url_template.format(&[sub_id]))?;

            let response = reqwest::Client::new()
                .get(url.clone())
                .header(
                    "Authorization",
                    format!("Bearer {}", response.token.secret()),
                )
                .send()
                .await?
                .text()
                .await?;

            let rt: ReturnType = serde_json::from_str(&response)?;
            collection.push(rt);
        }
        Ok(collection)
    }

    pub async fn rest_request_subscription_iter(
        &self,
        url_template: &str,
    ) -> Result<Vec<HecEvent>> {
        let response = self
            .credential
            .get_token("https://management.azure.com")
            .await?;

        let mut collection = vec![];
        for sub in self.subscriptions.inner.iter() {
            let sub_id = sub.subscription_id.as_ref().context("no sub id")?;
            let url = Url::parse(&url_template.format(&[sub_id]))?;
            dbg!(&url);
            let response = reqwest::Client::new()
                .get(url.clone())
                .header(
                    "Authorization",
                    format!("Bearer {}", response.token.secret()),
                )
                .send()
                .await?
                .text()
                .await?;

            let rt: ReturnType = serde_json::from_str(&response)?;
            dbg!(&rt);

            collection.extend(rt.to_hec_events(url.as_str())?);
        }
        Ok(collection)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum ReturnType {
    Collection {
        value: Vec<serde_json::Value>,
        #[serde(rename = "@odata.nextLink")]
        next_link: Option<String>,
    },
    Array(Vec<serde_json::Value>),
    Value(serde_json::Value),
}

// TODO Use ToHecEvents trait
impl ReturnType {
    pub fn to_hec_events(&self, source: &str) -> Result<Vec<HecEvent>> {
        let mut collection = vec![];
        match self {
            ReturnType::Collection { value, next_link } => {
                if let Some(next_link) = next_link {
                    dbg!(self);
                    dbg!(&next_link);
                    unimplemented!();
                }
                for v in value.iter() {
                    collection.push(HecEvent::new(
                        &v,
                        //&url.as_str(),
                        source,
                        v.as_object()
                            .context("value is not an object")?
                            .get("type")
                            .context("No key 'type'")?
                            .as_str()
                            .context("Type is not a str")?,
                    )?);
                }
            }
            ReturnType::Array(vec) => {
                for v in vec.iter() {
                    collection.push(HecEvent::new(
                        &v,
                        source,
                        //&url.as_str(),
                        v.as_object()
                            .context("value is not an object")?
                            .get("type")
                            .context("No key 'type'")?
                            .as_str()
                            .context("Type is not a str")?,
                    )?);
                }
            }
            ReturnType::Value(value) => {
                collection.push(HecEvent::new(
                    &value,
                    source,
                    //&url.as_str(),
                    value
                        .as_object()
                        .context("value is not an object")?
                        .get("type")
                        .context("No key 'type'")?
                        .as_str()
                        .context("Type is not a str")?,
                )?);
            }
        };
        Ok(collection)
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use crate::{
        azure_rest::Subscriptions,
        keyvault::get_keyvault_secrets,
        splunk::{set_ssphp_run, Splunk},
    };
    use anyhow::Result;

    use super::AzureRest;

    async fn setup() -> Result<(AzureRest, Splunk)> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME").unwrap())
            .await
            .unwrap();
        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
        set_ssphp_run()?;
        let azure_rest = AzureRest::new(
            &secrets.azure_client_id,
            &secrets.azure_client_secret,
            &secrets.azure_tenant_id,
        )
        .await?;

        Ok((azure_rest, splunk))
    }

    #[tokio::test]
    async fn test_azureclient_list_subscriptions() -> Result<()> {
        let (azure_rest, _splunk) = setup().await?;
        let subscriptions: Subscriptions = azure_rest.azure_subscriptions().await?;
        assert!(!subscriptions.inner.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_azureclient_security_contacts() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_security_contacts().await?;
        assert!(!collection.is_empty());
        splunk.send_batch(&collection[..]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_security_center_built_in() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_security_center_built_in().await?;
        assert!(!collection.is_empty());
        splunk.send_batch(&collection[..]).await?;
        Ok(())
    }

    // #[tokio::test]
    // async fn test_azureclient_get_microsoft_authorization_role_definitions() -> Result<()> {
    //     let (azure_rest, splunk) = setup().await?;
    //     let collection = azure_rest
    //         .get_microsoft_authorization_role_definitions()
    //         .await?;
    //     assert!(!collection.is_empty());
    //     splunk.send_batch(&collection[..]).await?;
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn test_azureclient_get_microsoft_authorization_role_assignments() -> Result<()> {
    //     let (azure_rest, splunk) = setup().await?;
    //     let collection = azure_rest
    //         .get_microsoft_authorization_role_assignments()
    //         .await?;
    //     splunk.send_batch(&collection[..]).await?;
    //     assert!(!collection.is_empty());
    //     Ok(())
    // }

    // 2.1.15
    #[tokio::test]
    async fn test_azureclient_get_microsoft_security_auto_provisioning_settings() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest
            .get_microsoft_security_auto_provisioning_settings()
            .await?;
        splunk.send_batch(&collection[..]).await?;
        assert!(!collection.is_empty());
        Ok(())
    }

    // 2.1.21
    // 2.1.22
    #[tokio::test]
    async fn test_azureclient_get_microsoft_security_settings() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_microsoft_security_settings().await?;
        splunk.send_batch(&collection[..]).await?;
        assert!(!collection.is_empty());
        Ok(())
    }

    // 4.1.3
    #[ignore]
    #[tokio::test]
    async fn test_azureclient_get_microsoft_sql_encryption_protection() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_microsoft_sql_encryption_protection().await?;
        splunk.send_batch(&collection[..]).await?;
        assert!(!collection.is_empty());
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RoleAssignment(SDKRoleAssignment);

impl RoleAssignment {
    pub fn role_definition_id(&self) -> Option<&String> {
        Some(&self.0.properties.as_ref()?.role_definition_id)
    }

    pub fn principal_type(&self) -> Option<&PrincipalType> {
        self.0.properties.as_ref()?.principal_type.as_ref()
    }

    pub fn principal_id(&self) -> Option<&String> {
        Some(&self.0.properties.as_ref()?.principal_id)
    }
}

impl<'a> ToHecEvents<'a> for HashMap<String, RoleAssignment> {
    type Item = RoleAssignment;
    fn source() -> &'static str {
        "azure_rest"
    }

    fn sourcetype() -> &'static str {
        "SSPHP.azure.role_assignment"
    }
    fn collection(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.values())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RoleDefinition(SDKRoleDefinition);
impl RoleDefinition {
    pub fn role_name(&self) -> Option<&String> {
        self.0.properties.as_ref()?.role_name.as_ref()
    }

    pub fn id(&self) -> Option<&String> {
        self.0.id.as_ref()
    }
}

impl<'a> ToHecEvents<'a> for HashMap<String, RoleDefinition> {
    type Item = RoleDefinition;
    fn source() -> &'static str {
        "azure_rest"
    }

    fn sourcetype() -> &'static str {
        "SSPHP.azure.role_definitions"
    }
    fn collection(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.values())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Subscriptions {
    inner: Vec<Subscription>,
}

impl<'a> ToHecEvents<'a> for Subscriptions {
    type Item = Subscription;
    fn source() -> &'static str {
        "azure_rest"
    }

    fn sourcetype() -> &'static str {
        "SSPHP.azure.subscriptions"
    }
    fn collection(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.inner.iter())
    }
}
