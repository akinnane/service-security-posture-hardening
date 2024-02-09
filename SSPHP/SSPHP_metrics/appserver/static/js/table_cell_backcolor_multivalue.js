require([
    'underscore',
    'jquery',
    'splunkjs/mvc',
    'splunkjs/mvc/tableview',
    'splunkjs/mvc/simplexml/ready!'
], function (_, $, mvc, TableView) {

    var CustomRangeRenderer = TableView.BaseCellRenderer.extend({
        canRender: function (cell) {
            //return true;
            return _(['state',
                      'in_r53',
                      'total_no_periods',
                      'periods_no_cloudtrail',
                      'mfaDelete',
                      'acceptable_policy_keys',
                      'publicAccessBlockConfiguration.blockPublicAcls',
                      'publicAccessBlockConfiguration.blockPublicPolicy',
                      'publicAccessBlockConfiguration.ignorePublicAcls',
                      'publicAccessBlockConfiguration.restrictPublicBuckets',
                      'access_key_1_last_rotated',
                      'access_key_2_last_rotated',
                      'attached_policies{}.policy_arn',
                      'policies{}',
                      'BucketName',
                      'Principal',
                      'Condition.Bool.aws:SecureTransport',
                      'Effect',
                      'Action',
                      'defaultUserRolePermissions.allowedToCreateTenants',
                      'conditions.locations.includeLocations{}',
                      'conditions.locations.excludeLocations{}',
                      'conditions.users.includeUsers{}',
                      'conditions.users.includeGroups{}',
                      'conditions.clientAppTypes{}',
                      'conditions.applications.includeApplications{}',
                      'conditions.users.includeRoles{}',
                      'conditions.applications.includeApplications{}',
                      'conditions.signInRiskLevels{}',
                      'grantControls.authenticationStrength.id',
                      'grantControls.builtInControls{}',
                      'EnableFileFilter',
                      'PhishThresholdLevel',
                      'EnableMailboxIntelligenceProtection',
                      'EnableMailboxIntelligence',
                      'EnableSpoofIntelligence',
                      'Enable',
                      'Action',
                      'QuarantineTag',
                      'Redirect',
                      'SetSCL',
                      'SenderDomainIs',
                      'RedirectTo',
                      'CopyTo',
                      'BlindCopyTo',
                      'AddToRecipients',
                      'AutoForwardingMode',
                      'BccSuspiciousOutboundMail',
                      'NotifyOutboundSpam',
                      'NotifyOutboundSpamRecipients',
                      'BccSuspiciousOutboundAdditionalRecipients',
                      'isEnabled',
                      'Enabled',
                      'EnableInternalSenderAdminNotifications',
                      'InternalSenderAdminAddress',
                      'MailTipsAllTipsEnabled',
                      'MailTipsExternalRecipientsTipsEnabled',
                      'MailTipsGroupMetricsEnabled',
                      'MailTipsLargeAudienceThreshold',
                      'Severity',
                      'Category',
                      'NotificationEnabled',
                      'UserTags',
                      'Filter',
                      'EnablePriorityAccountProtection',
                      'dmarc{}',
                      'txtRecords{}',
                      'txtRecords',
                      'Mode',
                      'Type',
                      'Domains{}',
                      'ScanUrls',
                      'DisableUrlRewrite',
                      'EnableForInternalSenders',
                      'DeliverMessageAfterScan',
                      'EnableSafeLinksForEmail',
                      'EnableSafeLinksForOffice',
                      'EnableSafeLinksForTeams',
                      'TeamsLocationName',
                      'TeamsLocationException',
                      'AllowClickThrough',
                      'AllowDropBox',
                      'AllowEgnyte',
                      'AllowBox',
                      'AllowGoogleDrive',
                      'AllowShareFile',
                      'TrackClicks',
                      'AllowTeamsConsumer',
                      'AllowPublicUsers',
                      'AllowFederatedUsers',
                      'AllowedDomain',
                      'settings.isInOrgFormsPhishingScanEnabled',
                      'EnableATPForSPOTeamsODB',
                      'permissionGrantPolicyIdsAssignedToDefaultUserRole{}',
                      'implementationStatus',
                      'isTrusted',
                      'notifyReviewers',
                      'reviewers',
                      'RoleAssignee',
                      'Role',
                      'values{}.value',
                      'values{}.name',
                      'permissionGrantPolicyIdsAssignedToDefaultUserRole{}',
                      'defaultUserRolePermissions.allowedToCreateApps',
                      'guestUserRoleId',
                      'allowInvitesFrom',
                      'defaultUserRolePermissions.allowedToCreateSecurityGroups',
                      'properties.type',
                      'properties.enabled',
                      'properties.assignableScopes{}',
                      'properties.permissions{}.actions{}',
                      'properties.enforcementMode',
                      'properties.autoProvision',
                      'properties.alertNotifications.state',
                      'properties.notificationsByRole.state',
                      'properties.notificationsByRole.roles{}',
                      'properties.emails',
                      'properties.alertNotifications.minimalSeverity',
                      'permissions{}.actions{}',
                      'sessionControls.signInFrequency.isEnabled',
                      'sessionControls.signInFrequency.type',
                      'sessionControls.signInFrequency.value',
                      'sessionControls.persistentBrowser.isEnabled',
                      'sessionControls.persistentBrowser.mode',
                      'roleName',
                      'blockSubscriptionsIntoTenant',
                      'blockSubscriptionsLeavingTenant',
                      'kind',
                      'properties.serverKeyType',
                      'uri',
                      'ipRanges{}.cidrAddress',
                      'alternateContactType',
                      'emailAddress',
                      'name',
                      'phoneNumber',
                      'SummaryMap.AccountAccessKeysPresent',
                      'SummaryMap.AccountMFAEnabled',
                      'serialNumber',
                      'SummaryMap.AccountMFAEnabled',
                      'number_with_AccountMFAEnabled',
                      'MinimumPasswordLength',
                      'PasswordReusePrevention',
                      'password_enabled',
                      'mfa_active',
                      'access_key_1_last_used_date',
                      'password_enabled',
                      'password_last_used',
                      'password_last_changed',
                      'password_days',
                      'password_compliant',
                      'access_key_1_active',
                      'access_key_1_last_used_date',
                      'access_key_1_last_rotated',
                      'access_key_1_days',
                      'access_key_1_compliant',
                      'access_key_2_active',
                      'access_key_2_last_used_date',
                      'access_key_2_last_rotated',
                      'access_key_2_days',
                      'access_key_2_compliant',
                      'implementationStatus',
                      'state',
                      'role',
                      'displayName',
                      'id_state',
                      'name_val',
                      'visibility',
                      'displayLocationInformationRequiredState.state',
                      'displayAppInformationRequiredState.state',
                      'assignmentType',
                      'B2BManagementPolicy.InvitationsAllowedAndBlockedDomainsPolicy.AllowedDomains{}',
                      'defaultUserRolePermissions.allowedToCreateTenants',
                      'OAuth2ClientProfileEnabled',
                      'passwordValidityPeriodInDays',
                      'onPremisesSyncEnabled',
                      'assignedPlans{}.servicePlanId',
                      'on',
                      'sourcetypes',
                      'eop_AntiPhishPolicy',
                      'phish_Id',
                      'eop_HostedContentFilterPolicy',
                      'eop_Identity',
                      'eop_MalwareFilterPolicy',
                      'eop_State',
                      'host_Id',
                      'mal_Id',
                      'sap_Id',
                      'slp_Id',
                      'scope.principalScopes{}.query',
                      'reviewers{}.query',
                      'settings.mailNotificationsEnabled',
                      'settings.reminderNotificationsEnabled',
                      'settings.recurrence.range.endDate',
                      'settings.recurrence.range.type',
                      'settings.recurrence.pattern.type',
                      'settings.recurrence.pattern.interval',
                      'settings.autoApplyDecisionsEnabled',
                      'settings.defaultDecision',
                      'settings.justificationRequiredOnApproval',
                      'resourceScopes',
                      'scope.principalScopes{}.query',
                      'reviewers{}.query',
                      'settings.autoApplyDecisionsEnabled',
                      'settings.defaultDecision',
                      'settings.justificationRequiredOnApproval',
                      'settings.mailNotificationsEnabled',
                      'settings.reminderNotificationsEnabled',
                      'settings.instanceDurationInDays',
                      'settings.recurrence.range.endDate',
                      'settings.recurrence.pattern.type',
                      'settings.recurrence.pattern.interval',
                      'UnifiedAuditLogIngestionEnabled',
                      'AuditDisabled',
                      'ThirdPartyFileProvidersEnabled',
                      'AdditionalStorageProvidersAvailable',
                      'title']).contains(cell.field);
        },
        render: function ($td, cell) {
            // Treat everything as a Arrays
            if (typeof cell.value == 'string') {
                cell.value = [cell.value];
            }
            
            var arrlen = cell.value.length;
            
            // Extract custom CSS class from last element.
            // What happens when the last value doesn't have a '|'?
            var css_class = cell.value[arrlen - 1].split("|")[1];
            
            // Add the CSS class to the TD
            $td.addClass("css_for_".concat(css_class));
            
            // Remove the CSS class  from the last element of the array
            cell.value[arrlen - 1] = cell.value[arrlen - 1].split("|")[0];
            
            // Join the array together with '\n' for display
            var label = cell.value.join("\n");
            $td.text(label);
            
            // Dunno why this is here
            //$td.addClass("multivalue-subcell");
            //$td.addClass("range-cell")
        }
    });

    var sh1 = mvc.Components.get("table3");
    if (typeof (sh1) != "undefined") {
        sh1.getVisualization(function (tableView) {
            // Add custom cell renderer and force re-render
            tableView.table.addCellRenderer(new CustomRangeRenderer());
            tableView.table.render();
        });
    }

    var sh2 = mvc.Components.get("table4");
    if (typeof (sh2) != "undefined") {
        sh2.getVisualization(function (tableView) {
            // Add custom cell renderer and force re-render
            tableView.table.addCellRenderer(new CustomRangeRenderer());
            tableView.table.render();
        });
    }
});
