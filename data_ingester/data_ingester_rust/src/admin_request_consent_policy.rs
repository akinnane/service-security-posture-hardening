use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::splunk::HecEvent;
use crate::splunk::ToHecEvents;

impl ToHecEvents for &AdminRequestConsentPolicy {
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365"
    }

    type Item = AdminRequestConsentPolicy;
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    fn to_hec_events(&self) -> anyhow::Result<Vec<crate::splunk::HecEvent>> {
        Ok(vec![HecEvent::new(
            &self,
            self.source(),
            self.sourcetype(),
        )?])
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRequestConsentPolicy {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    pub is_enabled: bool,
    pub notify_reviewers: bool,
    pub reminders_enabled: bool,
    pub request_duration_in_days: i64,
    pub version: i64,
    pub reviewers: Vec<AdminRequestConsentPolicyReviewer>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRequestConsentPolicyReviewer {
    pub query: String,
    pub query_type: String,
    pub query_root: Value,
}
