use crate::azure_rest::RoleAssignment;
use crate::azure_rest::RoleDefinition;
use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::conditional_access_policies::UserConditionalAccessPolicy;
use crate::directory_roles::DirectoryRole;
use crate::directory_roles::DirectoryRoles;
use crate::groups::Group;
use crate::groups::Groups;
use crate::roles::RoleDefinitions;
use crate::splunk::ToHecEvents;
use anyhow::Context;
use anyhow::Result;
use azure_mgmt_authorization::models::role_assignment_properties::PrincipalType;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::ops::Deref;

// https://learn.microsoft.com/en-us/graph/api/resources/user?view=graph-rest-1.0
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct User<'a> {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    assigned_plans: Vec<AssignedPlan>,
    // business_phones: Option<Vec<String>>,
    pub(crate) display_name: Option<String>,
    given_name: Option<String>,
    //job_title: Option<String>,
    mail: Option<String>,
    //mobile_phone: Option<String>,
    //office_location: Option<String>,
    //preferred_language: Option<String>,
    surname: Option<String>,
    user_principal_name: Option<String>,
    // Requires scope: AuditLog.Read.All
    sign_in_activity: Option<String>,
    account_enabled: Option<bool>,
    pub(crate) transitive_member_of: Option<Vec<GroupOrRole>>,
    #[serde(skip_deserializing)]
    conditional_access_policies: Option<Vec<UserConditionalAccessPolicy<'a>>>,
    // TODO!
    is_privileged: Option<bool>,
    // TODO! This might expand the json too much?
    pub azure_roles: Option<UserAzureRoles>,
    pub(crate) on_premises_sync_enabled: Option<bool>,
    // TODO!
    // assigned_plans: Option<???>,
}

// impl From for User {
//     fn from(span: Span) -> Self {
//         <Option as From>::from(span)
//     }
// }

/// Used to represent an AAD users roles in Azure (Cloud) subscriptions
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserAzureRoles {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub privileged_roles: Vec<UserAzureRole>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<UserAzureRole>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UserAzureRole {
    pub id: String,
    pub role_name: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssignedPlan {
    assigned_date_time: String,
    // TODO! ignroe Deleted & other...
    capability_status: String, //AssignedPlanCapabilityStatus,
    service: String,
    service_plan_id: String,
}

impl AssignedPlan {
    fn is_enabled(&self) -> bool {
        self.capability_status == "Enabled"
        // match self.capability_status {
        //     AssignedPlanCapabilityStatus::Enabled => true,
        //     AssignedPlanCapabilityStatus::Deleted => false,
        // }
    }
}

// #[derive(Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// enum AssignedPlanCapabilityStatus {
//     Enabled,
//     Deleted,
// }

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "@odata.type")]
pub(crate) enum GroupOrRole {
    #[serde(rename = "#microsoft.graph.group")]
    Group(Group),
    #[serde(rename = "#microsoft.graph.directoryRole")]
    Role(DirectoryRole),
}

impl<'a> User<'a> {
    #[cfg(test)]
    pub fn new(id: String, display_name: String) -> Self {
        Self {
            id,
            assigned_plans: vec![],
            //business_phones: None,
            display_name: Some(display_name),
            given_name: None,
            //job_title: None,
            mail: None,
            //mobile_phone: None,
            //office_location: None,
            //preferred_language: None,
            surname: None,
            user_principal_name: None,
            sign_in_activity: None,
            account_enabled: None,
            transitive_member_of: None,
            conditional_access_policies: None,
            is_privileged: None,
            azure_roles: None,
            on_premises_sync_enabled: None,
        }
    }

    pub fn groups(&self) -> Groups {
        self.transitive_member_of
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|dir_object| match dir_object {
                GroupOrRole::Group(group) => Some(group),
                GroupOrRole::Role(_) => None,
            })
            .collect::<Groups>()
    }

    pub fn roles(&self) -> DirectoryRoles {
        self.transitive_member_of
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|dir_object| match dir_object {
                GroupOrRole::Group(_) => None,
                GroupOrRole::Role(role) => Some(role),
            })
            .collect::<DirectoryRoles>()
    }

    pub fn set_is_privileged(&mut self, role_definitions: &RoleDefinitions) {
        for role in self.roles().value.iter() {
            match role_definitions.value.get(&role.role_template_id) {
                Some(role_definition) => {
                    if *role_definition.is_privileged.as_ref().unwrap_or(&false) {
                        self.is_privileged = Some(true);
                        return;
                    }
                }
                None => continue,
            }
        }
        self.is_privileged = Some(false)
    }

    pub fn assigned_plans_remove_deleted(&mut self) {
        self.assigned_plans.retain(|plan| plan.is_enabled());
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UsersMap<'a> {
    pub(crate) inner: HashMap<String, User<'a>>,
}

impl<'a> UsersMap<'a> {
    pub fn process_caps(&mut self, caps: &'a ConditionalAccessPolicies) {
        for (_, user) in self.inner.iter_mut() {
            let mut affected_caps = vec![];
            for cap in caps.value.iter() {
                if cap.affects_user(user) {
                    affected_caps.push(cap.to_user_conditional_access_policy())
                }
            }
            user.conditional_access_policies = Some(affected_caps)
        }
    }

    pub fn set_is_privileged(&mut self, role_definitions: &RoleDefinitions) {
        for (_, user) in self.inner.iter_mut() {
            user.set_is_privileged(role_definitions);
        }
    }

    pub fn extend_from_users(&mut self, users: Users<'a>) -> Result<()> {
        for user in users.value.into_iter() {
            self.inner
                .insert(user.id.to_string(), user)
                .context("Unable to insert User into UserMap")?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn extend(&mut self, users: UsersMap<'a>) {
        self.inner.extend(users.inner);
    }

    pub fn add_azure_roles(
        &mut self,
        role_assignments: &HashMap<String, RoleAssignment>,
        role_definitions: &HashMap<String, RoleDefinition>,
    ) -> Result<()> {
        let admin_roles_regex = Regex::new(r"(?i)(Owner|contributor|admin)").unwrap();

        for (_, role_assignment) in role_assignments.iter() {
            match &role_assignment
                .principal_type()
                .context("Principal Type not User")?
            {
                PrincipalType::User => {}
                _ => continue,
            }

            let role_assignment_role_definition_id = &role_assignment
                .role_definition_id()
                .context("No Role definition")?;

            let Some(role_definition) = role_definitions.get(*role_assignment_role_definition_id)
            else {
                continue;
            };

            let principal_id = &role_assignment.principal_id().context("No Principal ID")?;

            let Some(ref mut user) = self.inner.get_mut(*principal_id) else {
                continue;
            };

            let id = role_definition.id().context("no role id")?.to_string();

            let role_name = role_definition
                .role_name()
                .context("no role name")?
                .to_string();

            // TODO Should this be part of UserAzureRole?
            let priviliged = admin_roles_regex.find(&role_name).is_some();

            let azure_role = UserAzureRole { id, role_name };

            if user.azure_roles.is_none() {
                user.azure_roles = Some(UserAzureRoles::default());
            }

            if priviliged {
                user.azure_roles
                    .as_mut()
                    .unwrap()
                    .privileged_roles
                    .push(azure_role);
            } else {
                user.azure_roles.as_mut().unwrap().roles.push(azure_role);
            }
        }
        Ok(())
    }
}

impl<'a> ToHecEvents<'a> for UsersMap<'a> {
    type Item = User<'a>;
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "SSPHP.AAD.user"
    }
    fn collection(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.inner.values())
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Users<'a> {
    pub value: Vec<User<'a>>,
}

impl<'a> ToHecEvents<'a> for Users<'a> {
    type Item = User<'a>;
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "SSPHP.AAD.user"
    }
    fn collection(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.value.iter())
    }
}

impl<'a> Deref for Users<'a> {
    type Target = [User<'a>];

    fn deref(&self) -> &Self::Target {
        &self.value[..]
    }
}
