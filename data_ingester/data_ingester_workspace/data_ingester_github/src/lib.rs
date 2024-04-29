//! Pull Security posture data from the Github API and send it to a Splunk HEC.
//! Uses [Octocrab] for most operations.
pub mod entrypoint;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use data_ingester_supporting::keyvault::GitHubApp;
use http_body_util::BodyExt;
use itertools::Itertools;
use octocrab::models::{InstallationId, Repository};
use octocrab::Octocrab;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

/// NewType for Octocrab provide additonal data source.
#[derive(Clone)]
pub(crate) struct OctocrabGit {
    client: Octocrab,
}

impl OctocrabGit {
    /// Build an Octocrab client from a [data_ingester_supporting::keyvault::GitHubPat]
    // pub fn new_from_pat(github_pat: &GitHubPat) -> Result<Self> {
    //     let octocrab = Octocrab::builder()
    //         .personal_token(github_pat.pat.to_string())
    //         .build()?;
    //     Ok(Self { client: octocrab })
    // }

    pub async fn for_installation_id(&self, installation_id: InstallationId) -> Result<Self> {
        let (installation_client, _secret) =
            self.client.installation_and_token(installation_id).await?;
        Ok(Self {
            client: installation_client,
        })
    }

    fn new_from_app(github_app: &GitHubApp) -> Result<Self> {
        let key = jsonwebtoken::EncodingKey::from_rsa_der(&github_app.private_key); // .context("Building jsonwebtoken from gihtub app der key")?;

        let octocrab = Octocrab::builder()
            .app(github_app.app_id.into(), key)
            .build()
            .context("building Octocrab client for app")?;
        Ok(Self { client: octocrab })
    }

    /// Get a full list of [Repos] for the provided organization
    pub(crate) async fn org_repos(&self, org: &str) -> Result<Repos> {
        let page = self
            .client
            .orgs(org)
            .list_repos()
            .send()
            .await
            .context("getting org repos")?;
        let repos = self
            .client
            .all_pages(page)
            .await
            .context("getting additional org repo pages")?;
        Ok(Repos::new(repos, org))
    }

    /// Get the settings for the org
    pub(crate) async fn org_settings(&self, org: &str) -> Result<GithubResponses> {
        let uri = format!("/orgs/{org}");
        self.get_collection(&uri).await
    }

    /// Get all members for the organization
    pub(crate) async fn org_members(&self, org: &str) -> Result<GithubResponses> {
        let uri = format!("/orgs/{org}/members");
        self.get_collection(&uri).await
    }

    /// Get branch protection for a specific repo & branch
    pub(crate) async fn repo_branch_protection(
        &self,
        repo: &str,
        branch: &str,
    ) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/branches/{branch}/protection");
        self.get_collection(&uri).await
    }

    /// Get Dependabot alerts for a repo
    pub(crate) async fn repo_dependabot_alerts(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/dependabot/alerts");
        self.get_collection(&uri).await
    }

    /// Get the dependabot status for a repo
    pub(crate) async fn repo_dependabot_status(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/vulnerability-alerts");
        self.get_collection(&uri).await
    }

    /// Get deploy keys for a repo
    pub(crate) async fn repo_deploy_keys(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/keys");
        self.get_collection(&uri).await
    }

    pub(crate) async fn repo_code_scanning_default_setup(
        &self,
        repo: &str,
    ) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/code-scanning/default-setup");
        self.get_collection(&uri).await
    }

    pub(crate) async fn repo_codeowners(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/codeowners/errors");
        self.get_collection(&uri).await
    }

    pub(crate) async fn repo_secret_scanning_alerts(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/secret-scanning/alerts");
        self.get_collection(&uri).await
    }

    pub(crate) async fn repo_security_txt(&self, repo: &str) -> Result<GithubResponses> {
        let uris = [
            format!("/repos/{repo}/contents/SECURITY.md"),
            format!("/repos/{repo}/contents/.github/SECURITY.md"),
            format!("/repos/{repo}/contents/docs/SECURITY.md"),
        ];
        let mut responses = vec![];
        for uri in uris {
            let collection = self.get_collection(&uri).await?;
            if collection
                .inner
                .iter()
                .any(|response| response.ssphp_http_status == 200)
            {
                responses.extend(collection.inner);
                break;
            }
        }
        Ok(GithubResponses { inner: responses })
    }

    /// Get a relative uri from api.github.com and exhaust all next links.
    ///
    /// Returns all requests as seperate entries complete with status codes
    async fn get_collection(&self, uri: &str) -> Result<GithubResponses> {
        let mut next_link = GithubNextLink::from_str(uri);

        let mut responses = vec![];

        while let Some(next) = next_link.next {
            let response = self.client._get(next).await.context("Get url")?;

            next_link = GithubNextLink::from_response(&response)
                .await
                .context("Failed getting response 'link'")?;

            let status = response.status().as_u16();
            let mut body = response
                .collect()
                .await
                .context("collect body")?
                .to_bytes()
                .slice(0..);

            if body.is_empty() {
                body = "{}".into();
            }

            let body = match serde_json::from_slice(&body).context("Deserialize body") {
                Ok(ok) => ok,
                Err(err) => {
                    let s = String::from_utf8_lossy(&body);
                    dbg!(s);
                    anyhow::bail!(err);
                }
            };

            responses.push(GithubResponse {
                response: body,
                source: uri.to_string(),
                ssphp_http_status: status,
            });
        }

        Ok(GithubResponses { inner: responses })
    }
}

#[cfg(test)]
mod test {
    use std::{borrow::Borrow, env};

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;
    use futures::future::{BoxFuture, FutureExt};

    use crate::{OctocrabGit, Repos};

    use tokio::sync::OnceCell;

    #[derive(Clone)]
    struct TestClient {
        client: OctocrabGit,
        org_name: String,
        repos: Repos,
        splunk: Splunk,
    }
    static CLIENT: OnceCell<TestClient> = OnceCell::const_new();

    impl TestClient {
        async fn new() -> &'static Self {
            CLIENT
                .get_or_init(|| async {
                    TestClient::setup_app()
                        .await
                        .expect("Github Test Client should complete setup")
                })
                .await
        }

        async fn setup_app() -> Result<TestClient> {
            let secrets = get_keyvault_secrets(
                &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
            )
            .await
            .unwrap();
            let splunk = Splunk::new(
                &secrets.splunk_host.as_ref().context("No value")?,
                &secrets.splunk_token.as_ref().context("No value")?,
            )?;

            let github_app = secrets
                .github_app
                .as_ref()
                .expect("Github App should be configured");
            let client = OctocrabGit::new_from_app(github_app).context("Build OctocrabGit")?;

            let installations = client
                .client
                .apps()
                .installations()
                .send()
                .await
                .context("Getting installations for github app")?;
            for installation in installations {
                if installation.account.r#type != "Organization" {
                    continue;
                }
                let installation_client = client
                    .for_installation_id(installation.id)
                    .await
                    .context("build octocrabgit client")?;
                let org_name = &installation.account.login.to_string();

                let org_repos = installation_client
                    .org_repos(org_name)
                    .await
                    .context("Getting repos for org")?;
                return Ok(TestClient {
                    client: installation_client,
                    org_name: org_name.to_string(),
                    repos: org_repos,
                    splunk,
                });
            }
            anyhow::bail!("no github client created");
        }

        fn repo_name(&self, repo_name: &str) -> String {
            format!("{}/{}", &self.org_name, &repo_name)
        }

        async fn repo_iter<F, T, H>(&self, func: F) -> Result<()>
        where
            F: FnOnce(&str) -> BoxFuture<'_, Result<T>> + Copy,
            T: std::fmt::Debug + Borrow<H>,
            for<'a> &'a H: ToHecEvents,
        {
            for repo in self.repos.inner.iter() {
                let repo_name = self.repo_name(&repo.name);
                let result = func(&repo_name).await.unwrap();
                dbg!(&result);

                let events = result
                    .borrow()
                    .to_hec_events()
                    .context("ToHecEvents Serialize")?;
                dbg!(&events);

                self.splunk
                    .send_batch(events)
                    .await
                    .context("Sending events to Splunk")?;
            }
            Ok(())
        }

        async fn org<F, T, H>(&self, func: F) -> Result<()>
        where
            F: FnOnce(&str) -> BoxFuture<'_, Result<T>> + Copy,
            T: std::fmt::Debug + Borrow<H>,
            for<'a> &'a H: ToHecEvents,
        {
            let result = func(&self.org_name).await.unwrap();
            dbg!(&result);

            let events = result
                .borrow()
                .to_hec_events()
                .context("ToHecEvents Serialize")?;
            dbg!(&events);

            self.splunk
                .send_batch(events)
                .await
                .context("Sending events to Splunk")?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_github_repo_code_scanning() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| {
                client
                    .client
                    .repo_code_scanning_default_setup(&repo_name)
                    .boxed()
            })
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_codeowners() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| client.client.repo_codeowners(&repo_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_deploy_keys() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| client.client.repo_deploy_keys(&repo_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_dependabot_status() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| client.client.repo_dependabot_status(&repo_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_dependabot_alerts() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| client.client.repo_dependabot_alerts(&repo_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_secret_scanning() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| {
                client
                    .client
                    .repo_secret_scanning_alerts(&repo_name)
                    .boxed()
            })
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_org_settings() -> Result<()> {
        let client = TestClient::new().await;
        client
            .org(|org_name: &str| client.client.org_settings(&org_name).boxed())
            .await?;
        Ok(())
    }
}

#[derive(Serialize, Debug, Clone)]
struct Repos {
    inner: Vec<Repository>,
    source: String,
}

/// New type for Vec<[Repository]> including the source of the repository
impl Repos {
    fn new(repos: Vec<Repository>, org: &str) -> Self {
        Self {
            inner: repos,
            source: format!("github:{}", org),
        }
    }
}

impl ToHecEvents for &Repos {
    type Item = Repository;
    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        "github:repository"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

/// A collection of API responses from Github
#[derive(Serialize, Debug)]
struct GithubResponses {
    inner: Vec<GithubResponse>,
}

impl ToHecEvents for &GithubResponses {
    type Item = GithubResponse;

    /// Not used
    fn source(&self) -> &str {
        unimplemented!()
    }

    /// Not used
    fn sourcetype(&self) -> &str {
        unimplemented!()
    }

    /// Not used
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    /// Create a collection of
    /// [data_ingester_splunk::splunk::HecEvent] for each element in
    /// of a Github response, in a collection of github responses.
    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        Ok(self
            .inner
            .iter()
            .flat_map(|response| response.to_hec_events())
            .flatten()
            .collect())
    }
}

/// An  API responses from Github
#[derive(Serialize, Debug)]
struct GithubResponse {
    #[serde(flatten)]
    response: SingleOrVec,
    #[serde(skip)]
    source: String,
    ssphp_http_status: u16,
}

/// Descriminator type to help [serde::Deserialize] deal with API endpoints that return a '{}' or a '[{}]'
#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum SingleOrVec {
    Vec(Vec<serde_json::Value>),
    Single(serde_json::Value),
}

impl ToHecEvents for &GithubResponse {
    type Item = Self;
    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    /// Not used
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    /// Create a [data_ingester_splunk::splunk::HecEvent] for each
    /// element of a collection returned by a single GitHub api call.
    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        // TODO FIX THIS
        // Shouldn't have to clone all the values :(
        let data = match &self.response {
            SingleOrVec::Single(single) => vec![single.clone()],
            SingleOrVec::Vec(vec) => vec.to_vec(),
        };

        let (ok, _err): (Vec<_>, Vec<_>) = data
            .iter()
            .map(|event| GithubResponse {
                response: SingleOrVec::Single(event.clone()),
                source: self.source.clone(),
                ssphp_http_status: self.ssphp_http_status,
            })
            .map(|gr| {
                data_ingester_splunk::splunk::HecEvent::new(&gr, self.source(), self.sourcetype())
            })
            .partition_result();
        Ok(ok)
    }
}

/// Helper for paginating GitHub resoponses.
///
/// Represents the link to the next page of results for a paginated Github request.
///
/// The link is stored as just the path and query elements of the URI
/// for compatibility with OctoCrab authentication
///
#[derive(Debug)]
struct GithubNextLink {
    next: Option<String>,
}

impl GithubNextLink {
    /// Use the exact url as the next link
    fn from_str(url: impl Into<String>) -> Self {
        Self {
            next: Some(url.into()),
        }
    }

    /// Take a `link` header, as returned  by Github, and create a new [GithubNextLink] from it.
    async fn from_link_str(header: &str) -> Self {
        static CELL: OnceCell<Regex> = OnceCell::const_new();
        let regex = CELL
            .get_or_init(|| async {
                Regex::new(r#"<(?<url>[^>]+)>; rel=\"next\""#).expect("Regex is valid")
            })
            .await;

        let next = regex
            .captures(header)
            .and_then(|cap| cap.name("url").map(|m| m.as_str().to_string()))
            .and_then(|url| http::uri::Uri::from_maybe_shared(url).ok())
            .and_then(|uri| uri.path_and_query().map(|pq| pq.as_str().to_string()));

        Self { next }
    }

    /// Create a next link from a [http::Response] from GitHub API.
    async fn from_response<T>(response: &http::Response<T>) -> Result<Self> {
        let header = if let Some(header) = response.headers().get("link") {
            header
                .to_str()
                .context("Unable to parse GitHub link header")?
        } else {
            return Ok(Self { next: None });
        };
        Ok(Self::from_link_str(header).await)
    }
}

#[cfg(test)]
mod test_github_next_link {
    use anyhow::Result;

    use crate::GithubNextLink;
    #[tokio::test]
    async fn test_github_links() -> Result<()> {
        let header = "<https://api.github.com/repositories/123456789/dependabot/alerts?per_page=1&page=2>; rel=\"next\", <https://api.github.com/repositories/123456789/dependabot/alerts?per_page=1&page=5>; rel=\"last\"";

        let next = GithubNextLink::from_link_str(header).await;
        assert!(next.next.is_some());
        assert_eq!(
            next.next.unwrap(),
            "/repositories/123456789/dependabot/alerts?per_page=1&page=2".to_string()
        );
        Ok(())
    }
}
