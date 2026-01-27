use anyhow::Result;
use crossterm::{event::Event, execute, terminal::*};
use ratatui::prelude::*;
use rusticity_term::{App, EventHandler, Service, ViewMode};
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    rusticity_term::init();

    let mut terminal = setup_terminal()?;

    let app_result = App::new(None, None).await;

    let mut app = match app_result {
        Ok(app) => app,
        Err(e) => {
            let profile_name = std::env::var("AWS_PROFILE").ok();
            let mut app = App::new_without_client(profile_name.clone().unwrap_or_default(), None);
            let error_str = format!("{:#}", e);

            // If no profile specified, open profile picker
            if profile_name.is_none() || error_str.contains("No AWS profile specified") {
                app.available_profiles = App::load_aws_profiles();
                app.mode = rusticity_term::keymap::Mode::ProfilePicker;
            } else if error_str.contains("Missing Region") {
                app.mode = rusticity_term::keymap::Mode::RegionPicker;
            } else {
                app.error_message = Some(error_str);
                app.error_scroll = 0;
                app.mode = rusticity_term::keymap::Mode::ErrorModal;
            }
            app
        }
    };

    let events = EventHandler::new();

    terminal.draw(|f| rusticity_term::ui::render(f, &app))?;

    let mut tick_interval = interval(Duration::from_millis(100));

    while app.running {
        tokio::select! {
            _ = tick_interval.tick(), if app.log_groups_state.loading => {
                terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
            }
            event = async { events.next() } => {
                match event? {
                    Some(Event::Key(key)) => {
                        use crossterm::event::KeyCode;

                        // Handle 'g' prefix key combinations
                        if let Some(pending) = app.pending_key {
                            app.pending_key = None;
                            if pending == 'g' {
                                match key.code {
                                    KeyCode::Char('n') => {
                                        app.handle_action(rusticity_term::keymap::Action::NextTab);
                                        terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                        continue;
                                    },
                                    KeyCode::Char('p') => {
                                        app.handle_action(rusticity_term::keymap::Action::PrevTab);
                                        terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                        continue;
                                    },
                                    _ => {}, // Invalid combination, ignore
                                }
                            }
                        }

                        // Check if this is a 'g' prefix key
                        if app.mode == rusticity_term::keymap::Mode::Normal && key.code == KeyCode::Char('g') {
                            app.pending_key = Some('g');
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            continue;
                        }

                        if let Some(action) = rusticity_term::keymap::handle_key(key, app.mode) {
                        let should_load = matches!(action, rusticity_term::keymap::Action::Select);
                        let should_refresh = matches!(action, rusticity_term::keymap::Action::Refresh);
                        let should_retry = matches!(action, rusticity_term::keymap::Action::RetryLoad);
                        let is_profile_select = should_load && app.mode == rusticity_term::keymap::Mode::ProfilePicker;
                        let is_region_select = should_load && app.mode == rusticity_term::keymap::Mode::RegionPicker;
                        let is_profile_refresh = should_refresh && app.mode == rusticity_term::keymap::Mode::ProfilePicker;
                        let prev_loading = app.log_groups_state.loading;
                        let prev_view_mode = app.view_mode;
                        let prev_service_selected = app.service_selected;
                        let prev_service = app.current_service;
                        let prev_current_role = app.iam_state.current_role.clone();
                        let prev_current_group = app.iam_state.current_group.clone();
                        let prev_lambda_function = app.lambda_state.current_function.clone();
                        let prev_lambda_tab = app.lambda_state.detail_tab;

                        app.handle_action(action);

                        // Fetch profile accounts on Ctrl+R in profile picker
                        if is_profile_refresh {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            app.fetch_profile_accounts().await;
                            app.log_groups_state.loading = false;
                            app.log_groups_state.loading_message.clear();
                        }

                        // Load log groups when service is switched to and empty
                        if app.service_selected && app.current_service == Service::CloudWatchLogGroups
                            && (!prev_service_selected || prev_service != Service::CloudWatchLogGroups)
                            && app.log_groups_state.log_groups.items.is_empty() {
                            app.log_groups_state.loading = true;
                            app.log_groups_state.loading_message = "Loading log groups...".to_string();
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            let _ = app.load_log_groups().await;
                            app.log_groups_state.loading = false;
                            app.log_groups_state.loading_message.clear();
                        }

                        // Load S3 buckets when service is switched to and empty
                        if app.service_selected && app.current_service == Service::S3Buckets
                            && (!prev_service_selected || prev_service != Service::S3Buckets)
                            && app.s3_state.buckets.items.is_empty() {
                            app.s3_state.buckets.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::s3::load_s3_buckets(&mut app).await {
                                app.error_message = Some(format!("Failed to load S3 buckets: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.s3_state.buckets.loading = false;
                        }

                        // Load SQS queues when service is switched to and empty
                        if app.service_selected && app.current_service == Service::SqsQueues
                            && (!prev_service_selected || prev_service != Service::SqsQueues)
                            && app.sqs_state.queues.items.is_empty() {
                            app.sqs_state.queues.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::sqs::load_sqs_queues(&mut app).await {
                                app.error_message = Some(format!("Failed to load SQS queues: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.sqs_state.queues.loading = false;
                        }

                        // Load Lambda triggers when viewing queue detail on triggers tab
                        if app.current_service == Service::SqsQueues
                            && app.sqs_state.current_queue.is_some()
                            && app.sqs_state.detail_tab == rusticity_term::ui::sqs::QueueDetailTab::LambdaTriggers
                            && app.sqs_state.triggers.loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(queue_url) = &app.sqs_state.current_queue.clone() {
                                if let Err(e) = rusticity_term::ui::sqs::load_lambda_triggers(&mut app, queue_url).await {
                                    app.error_message = Some(format!("Failed to load Lambda triggers: {:#}", e));
                                    app.error_scroll = 0;
                                    app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                            app.sqs_state.triggers.loading = false;
                        }

                        // Load API Gateway routes when viewing API detail
                        if app.current_service == Service::ApiGatewayApis
                            && app.apig_state.current_api.is_some()
                            && app.apig_state.routes.loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(api) = &app.apig_state.current_api.clone() {
                                // Only load routes for HTTP/WebSocket APIs (v2), not REST APIs (v1)
                                let protocol = api.protocol_type.to_uppercase();
                                if protocol == "HTTP" || protocol == "WEBSOCKET" {
                                    if let Err(e) = rusticity_term::ui::apig::load_routes(&mut app, &api.id).await {
                                        app.error_message = Some(format!("Failed to load routes: {:#}", e));
                                        app.error_scroll = 0;
                                        app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                    }
                                } else {
                                    // REST APIs don't have routes - show message
                                    app.apig_state.routes.items.clear();
                                }
                            }
                            app.apig_state.routes.loading = false;
                        }

                        // Load API Gateway resources for REST APIs
                        if app.current_service == Service::ApiGatewayApis
                            && app.apig_state.current_api.is_some()
                            && app.apig_state.resources.loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(api) = &app.apig_state.current_api.clone() {
                                let protocol = api.protocol_type.to_uppercase();
                                if protocol == "REST" {
                                    if let Err(e) = rusticity_term::ui::apig::load_resources(&mut app, &api.id).await {
                                        app.error_message = Some(format!("Failed to load resources: {:#}", e));
                                        app.error_scroll = 0;
                                        app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                    }
                                } else {
                                    app.apig_state.resources.items.clear();
                                }
                            }
                            app.apig_state.resources.loading = false;
                        }

                        // Load metrics when viewing queue detail on monitoring tab
                        if app.current_service == Service::SqsQueues
                            && app.sqs_state.current_queue.is_some()
                            && app.sqs_state.detail_tab == rusticity_term::ui::sqs::QueueDetailTab::Monitoring
                            && app.sqs_state.metrics_loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(queue) = app.sqs_state.queues.items.iter().find(|q| Some(&q.url) == app.sqs_state.current_queue.as_ref()) {
                                let queue_name = queue.name.clone();
                                if let Err(e) = rusticity_term::ui::sqs::load_metrics(&mut app, &queue_name).await {
                                    app.error_message = Some(format!("Failed to load metrics: {:#}", e));
                                    app.error_scroll = 0;
                                    app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                            app.sqs_state.metrics_loading = false;
                        }

                        // Load EventBridge Pipes when tab is switched
                        if app.current_service == Service::SqsQueues
                            && app.sqs_state.current_queue.is_some()
                            && app.sqs_state.detail_tab == rusticity_term::ui::sqs::QueueDetailTab::EventBridgePipes
                            && app.sqs_state.pipes.loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(queue_url) = &app.sqs_state.current_queue.clone() {
                                if let Err(e) = rusticity_term::ui::sqs::load_pipes(&mut app, queue_url).await {
                                    app.error_message = Some(format!("Failed to load EventBridge Pipes: {:#}", e));
                                    app.error_scroll = 0;
                                    app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                            app.sqs_state.pipes.loading = false;
                        }

                        // Load Tags when tab is switched
                        if app.current_service == Service::SqsQueues
                            && app.sqs_state.current_queue.is_some()
                            && app.sqs_state.detail_tab == rusticity_term::ui::sqs::QueueDetailTab::Tagging
                            && app.sqs_state.tags.loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(queue_url) = &app.sqs_state.current_queue.clone() {
                                if let Err(e) = rusticity_term::ui::sqs::load_tags(&mut app, queue_url).await {
                                    app.error_message = Some(format!("Failed to load tags: {:#}", e));
                                    app.error_scroll = 0;
                                    app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                            app.sqs_state.tags.loading = false;
                        }

                        // Load CloudWatch Log Group tags when tab is switched
                        if app.current_service == Service::CloudWatchLogGroups
                            && app.view_mode == ViewMode::Detail
                            && app.log_groups_state.detail_tab == rusticity_term::ui::cw::logs::DetailTab::Tags
                            && app.log_groups_state.tags.loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_log_group_tags().await {
                                app.error_message = Some(format!("Failed to load tags: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.log_groups_state.tags.loading = false;
                        }

                        // Load CloudWatch Alarms when service is switched to and empty
                        if app.service_selected && app.current_service == Service::CloudWatchAlarms
                            && (!prev_service_selected || prev_service != Service::CloudWatchAlarms)
                            && app.alarms_state.table.items.is_empty() {
                            app.alarms_state.table.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_alarms().await {
                                app.error_message = Some(format!("Failed to load alarms: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.alarms_state.table.loading = false;
                        }

                        // Load EC2 instances when service is switched to, empty, or loading
                        if app.service_selected && app.current_service == Service::Ec2Instances
                            && ((!prev_service_selected || prev_service != Service::Ec2Instances)
                                || app.ec2_state.table.loading)
                        {
                            app.ec2_state.table.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_ec2_instances().await {
                                app.error_message = Some(format!("Failed to load EC2 instances: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.ec2_state.table.loading = false;
                        }

                        // Load EC2 tags when tab is switched
                        if app.current_service == Service::Ec2Instances
                            && app.ec2_state.current_instance.is_some()
                            && app.ec2_state.detail_tab == rusticity_term::ui::ec2::DetailTab::Tags
                            && app.ec2_state.tags.loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(instance_id) = &app.ec2_state.current_instance.clone() {
                                if let Err(e) = rusticity_term::ui::ec2::load_tags(&mut app, instance_id).await {
                                    app.error_message = Some(format!("Failed to load tags: {:#}", e));
                                    app.error_scroll = 0;
                                    app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                            app.ec2_state.tags.loading = false;
                        }

                        // Load EC2 metrics when entering Monitoring tab
                        if app.current_service == Service::Ec2Instances
                            && app.ec2_state.current_instance.is_some()
                            && app.ec2_state.detail_tab == rusticity_term::ui::ec2::DetailTab::Monitoring
                            && app.ec2_state.metrics_loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(instance_id) = &app.ec2_state.current_instance.clone() {
                                if let Err(e) = rusticity_term::ui::ec2::load_ec2_metrics(&mut app, instance_id).await {
                                    app.error_message = Some(format!("Failed to load metrics: {:#}", e));
                                    app.error_scroll = 0;
                                    app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                            app.ec2_state.metrics_loading = false;
                        }

                        // Load ECR repositories when service is switched to, empty, or loading
                        if app.service_selected && app.current_service == Service::EcrRepositories
                            && ((!prev_service_selected || prev_service != Service::EcrRepositories)
                                || app.ecr_state.repositories.loading)
                            && app.ecr_state.current_repository.is_none() {
                            app.ecr_state.repositories.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_ecr_repositories().await {
                                app.error_message = Some(format!("Failed to load ECR repositories: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.ecr_state.repositories.loading = false;
                        }

                        // Load ECR images when drilling into repository
                        if app.current_service == Service::EcrRepositories
                            && app.ecr_state.repositories.loading
                            && app.ecr_state.current_repository.is_some() {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_ecr_images().await {
                                app.error_message = Some(format!("Failed to load ECR images: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.ecr_state.repositories.loading = false;
                        }

                        // Load API Gateway APIs when service is switched to, empty, or loading
                        if app.service_selected && app.current_service == Service::ApiGatewayApis
                            && ((!prev_service_selected || prev_service != Service::ApiGatewayApis)
                                || app.apig_state.apis.loading)
                        {
                            app.apig_state.apis.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_apis().await {
                                app.error_message = Some(format!("Failed to load APIs: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.apig_state.apis.loading = false;
                        }

                        // Load Lambda functions
                        if app.service_selected && app.current_service == Service::LambdaFunctions
                            && ((!prev_service_selected || prev_service != Service::LambdaFunctions)
                                || app.lambda_state.table.loading) {
                            app.lambda_state.table.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::lambda::load_lambda_functions(&mut app).await {
                                app.error_message = Some(format!("Failed to load Lambda functions: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.lambda_state.table.loading = false;
                        }

                        // Load Lambda applications
                        if app.service_selected && app.current_service == Service::LambdaApplications
                            && ((!prev_service_selected || prev_service != Service::LambdaApplications)
                                || app.lambda_application_state.table.loading) {
                            app.lambda_application_state.table.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::lambda::load_lambda_applications(&mut app).await {
                                app.error_message = Some(format!("Failed to load Lambda applications: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.lambda_application_state.table.loading = false;
                        }

                        // Load CloudFormation stacks when service is switched to and empty
                        if app.service_selected && app.current_service == Service::CloudFormationStacks
                            && (!prev_service_selected || prev_service != Service::CloudFormationStacks)
                            && app.cfn_state.table.items.is_empty() {
                            app.cfn_state.table.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_cloudformation_stacks().await {
                                app.error_message = Some(format!("Failed to load CloudFormation stacks: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.cfn_state.table.loading = false;
                        }

                        // Load CFN template when stack is selected
                        if app.service_selected && app.current_service == Service::CloudFormationStacks
                            && app.cfn_state.current_stack.is_some()
                            && app.cfn_state.table.loading {
                            let stack_name = app.cfn_state.current_stack.clone().unwrap();
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_cfn_template(&stack_name).await {
                                app.error_message = Some(format!("Failed to load template: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            if let Err(e) = app.load_cfn_parameters(&stack_name).await {
                                app.error_message = Some(format!("Failed to load parameters: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            if let Err(e) = app.load_cfn_outputs(&stack_name).await {
                                app.error_message = Some(format!("Failed to load outputs: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            if let Err(e) = app.load_cfn_resources(&stack_name).await {
                                app.error_message = Some(format!("Failed to load resources: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.cfn_state.table.loading = false;
                        }

                        // Load IAM users when service is switched to and empty
                        if app.service_selected && app.current_service == Service::IamUsers
                            && (!prev_service_selected || prev_service != Service::IamUsers)
                            && app.iam_state.users.items.is_empty() {
                            app.iam_state.users.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::iam::load_iam_users(&mut app).await {
                                app.error_message = Some(format!("Failed to load IAM users: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.iam_state.users.loading = false;
                        }

                        // Load IAM roles when service is switched to and empty
                        if app.service_selected && app.current_service == Service::IamRoles
                            && (!prev_service_selected || prev_service != Service::IamRoles)
                            && app.iam_state.roles.items.is_empty() {
                            app.iam_state.roles.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::iam::load_iam_roles(&mut app).await {
                                app.error_message = Some(format!("Failed to load IAM roles: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.iam_state.roles.loading = false;
                        }

                        // Load IAM user groups when service is switched to and empty
                        if app.service_selected && app.current_service == Service::IamUserGroups
                            && (!prev_service_selected || prev_service != Service::IamUserGroups)
                            && app.iam_state.groups.items.is_empty() {
                            app.iam_state.groups.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::iam::load_iam_user_groups(&mut app).await {
                                app.error_message = Some(format!("Failed to load IAM user groups: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.iam_state.groups.loading = false;
                        }

                        // Load role policies when entering role detail view
                        if app.current_service == Service::IamRoles
                            && app.iam_state.current_role.is_some()
                            && prev_current_role != app.iam_state.current_role
                        {
                            let role_name = app.iam_state.current_role.clone().unwrap();
                            app.iam_state.policies.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_role_policies(&role_name).await {
                                app.error_message = Some(format!("Failed to load role policies: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            // Also load trust policy
                            if let Err(e) = app.load_trust_policy(&role_name).await {
                                app.error_message = Some(format!("Failed to load trust policy: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            // Also load last accessed services
                            if let Err(e) = app.load_last_accessed_services(&role_name).await {
                                app.error_message = Some(format!("Failed to load last accessed services: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.iam_state.policies.loading = false;
                        }

                        // Load group policies when entering group detail view
                        if app.current_service == Service::IamUserGroups
                            && app.iam_state.current_group.is_some()
                            && prev_current_group != app.iam_state.current_group
                        {
                            let group_name = app.iam_state.current_group.clone().unwrap();
                            app.iam_state.policies.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_group_policies(&group_name).await {
                                app.error_message = Some(format!("Failed to load group policies: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.iam_state.policies.loading = false;

                            // Also load group users
                            app.iam_state.group_users.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_group_users(&group_name).await {
                                app.error_message = Some(format!("Failed to load group users: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.iam_state.group_users.loading = false;
                        }

                        // Load Lambda versions when entering Versions tab
                        if app.current_service == Service::LambdaFunctions
                            && app.lambda_state.current_function.is_some()
                            && app.lambda_state.detail_tab == rusticity_term::app::LambdaDetailTab::Versions
                            && (prev_lambda_function != app.lambda_state.current_function
                                || prev_lambda_tab != app.lambda_state.detail_tab)
                        {
                            let function_name = app.lambda_state.current_function.clone().unwrap();
                            app.lambda_state.version_table.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::lambda::load_lambda_versions(&mut app, &function_name).await {
                                app.error_message = Some(format!("Failed to load Lambda versions: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.lambda_state.version_table.loading = false;
                        }

                        // Load Lambda aliases when entering Aliases tab
                        if app.current_service == Service::LambdaFunctions
                            && app.lambda_state.current_function.is_some()
                            && app.lambda_state.detail_tab == rusticity_term::app::LambdaDetailTab::Aliases
                            && (prev_lambda_function != app.lambda_state.current_function
                                || prev_lambda_tab != app.lambda_state.detail_tab)
                        {
                            let function_name = app.lambda_state.current_function.clone().unwrap();
                            app.lambda_state.alias_table.loading = true;
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = rusticity_term::ui::lambda::load_lambda_aliases(&mut app, &function_name).await {
                                app.error_message = Some(format!("Failed to load Lambda aliases: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.lambda_state.alias_table.loading = false;
                        }

                        // Load Lambda metrics when entering Monitoring tab
                        if app.current_service == Service::LambdaFunctions
                            && app.lambda_state.current_function.is_some()
                            && app.lambda_state.detail_tab == rusticity_term::app::LambdaDetailTab::Monitor
                            && app.lambda_state.metrics_loading
                        {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Some(function) = app.lambda_state.table.items.iter().find(|f| Some(&f.name) == app.lambda_state.current_function.as_ref()) {
                                let function_name = function.name.clone();
                                let version = app.lambda_state.current_version.clone();
                                if let Err(e) = rusticity_term::ui::lambda::load_lambda_metrics(&mut app, &function_name, version.as_deref()).await {
                                    app.error_message = Some(format!("Failed to load metrics: {:#}", e));
                                    app.error_scroll = 0;
                                    app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                            app.lambda_state.metrics_loading = false;
                        }

                        // Load tags when switching to Tags tab
                        if app.current_service == Service::IamRoles
                            && app.iam_state.current_role.is_some()
                            && app.iam_state.role_tab == rusticity_term::ui::iam::RoleTab::Tags
                            && app.iam_state.tags.loading
                        {
                            let role_name = app.iam_state.current_role.clone().unwrap();
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_role_tags(&role_name).await {
                                app.error_message = Some(format!("Failed to load role tags: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.iam_state.tags.loading = false;
                        }

                        // Load user tags when switching to Tags tab
                        if app.current_service == Service::IamUsers
                            && app.iam_state.current_user.is_some()
                            && app.iam_state.user_tab == rusticity_term::ui::iam::UserTab::Tags
                            && app.iam_state.user_tags.loading
                        {
                            let user_name = app.iam_state.current_user.clone().unwrap();
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.load_user_tags(&user_name).await {
                                app.error_message = Some(format!("Failed to load user tags: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.iam_state.user_tags.loading = false;
                        }

                        // Load policy document when entering policy view
                        if app.view_mode == ViewMode::PolicyView
                            && app.iam_state.policies.loading
                        {
                            let role_name = app.iam_state.current_role.clone();
                            let policy_name = app.iam_state.current_policy.clone();

                            if let (Some(role_name), Some(policy_name)) = (role_name, policy_name) {
                                terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                if let Err(e) = app.load_policy_document(&role_name, &policy_name).await {
                                    app.error_message = Some(format!("Failed to load policy document: {:#}", e));
                                    app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                                app.iam_state.policies.loading = false;
                            }
                        }

                        // Load S3 objects when drilling into bucket/prefix
                        if app.current_service == Service::S3Buckets && app.s3_state.buckets.loading {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;

                            // Check if we're loading preview or full objects
                            if app.s3_state.current_bucket.is_none() {
                                // Loading preview for expanded buckets (not prefixes)
                                let bucket_names: std::collections::HashSet<String> = app.s3_state.buckets.items.iter()
                                    .map(|b| b.name.clone())
                                    .collect();
                                let expanded_buckets: Vec<String> = app.s3_state.expanded_prefixes.iter()
                                    .filter(|name| bucket_names.contains(*name) && !app.s3_state.bucket_preview.contains_key(*name) && !app.s3_state.bucket_errors.contains_key(*name))
                                    .cloned()
                                    .collect();
                                for bucket_name in expanded_buckets {
                                    if let Err(e) = app.load_bucket_preview(bucket_name.clone()).await {
                                        let err_str = format!("{:#}", e);
                                        // Store error message to display in UI
                                        app.s3_state.bucket_errors.insert(bucket_name, err_str);
                                    }
                                }

                                // Load prefix previews from bucket preview
                                fn find_prefix_in_bucket(
                                    prefix: &str,
                                    _bucket_name: &str,
                                    bucket_preview: &Option<&Vec<rusticity_term::s3::Object>>,
                                    prefix_preview: &std::collections::HashMap<String, Vec<rusticity_term::s3::Object>>,
                                ) -> bool {
                                    // Check direct children in bucket
                                    if let Some(preview) = bucket_preview {
                                        if preview.iter().any(|o| o.is_prefix && o.key == prefix) {
                                            return true;
                                        }
                                    }
                                    // Check in nested prefix previews
                                    for (_, nested_preview) in prefix_preview.iter() {
                                        if nested_preview.iter().any(|o| o.is_prefix && o.key == prefix) {
                                            return true;
                                        }
                                    }
                                    false
                                }

                                // Load prefix previews - only for buckets that are expanded
                                let expanded_prefixes: Vec<(String, String)> = app.s3_state.expanded_prefixes.iter()
                                    .filter_map(|prefix| {
                                        // Skip if already loaded
                                        if app.s3_state.prefix_preview.contains_key(prefix) {
                                            return None;
                                        }
                                        // Find the bucket by checking which bucket has this prefix in its preview
                                        for bucket in &app.s3_state.buckets.items {
                                            // Skip if bucket not expanded
                                            if !app.s3_state.expanded_prefixes.contains(&bucket.name) {
                                                continue;
                                            }
                                            let bucket_preview = app.s3_state.bucket_preview.get(&bucket.name);
                                            if find_prefix_in_bucket(prefix, &bucket.name, &bucket_preview, &app.s3_state.prefix_preview) {
                                                return Some((bucket.name.clone(), prefix.clone()));
                                            }
                                        }
                                        None
                                    })
                                    .collect();

                                for (bucket, prefix) in expanded_prefixes {
                                    let _ = app.load_prefix_preview(bucket.clone(), prefix).await;
                                }
                            } else {
                                // Check if loading prefix preview or full objects
                                if app.s3_state.current_bucket.is_some() {
                                    // In bucket view - load prefix preview
                                    // Helper to recursively check if prefix exists in nested structure
                                    fn prefix_exists_in_tree(
                                        prefix: &str,
                                        objects: &[rusticity_term::s3::Object],
                                        prefix_preview: &std::collections::HashMap<String, Vec<rusticity_term::s3::Object>>,
                                    ) -> bool {
                                        // Check top-level objects
                                        if objects.iter().any(|o| o.is_prefix && o.key == prefix) {
                                            return true;
                                        }
                                        // Recursively check all previews
                                        for preview in prefix_preview.values() {
                                            if preview.iter().any(|o| o.is_prefix && o.key == prefix) {
                                                return true;
                                            }
                                        }
                                        false
                                    }

                                    let expanded_prefixes: Vec<String> = app.s3_state.expanded_prefixes.iter()
                                        .filter(|prefix| {
                                            !app.s3_state.prefix_preview.contains_key(*prefix) &&
                                            prefix_exists_in_tree(prefix, &app.s3_state.objects, &app.s3_state.prefix_preview)
                                        })
                                        .cloned()
                                        .collect();

                                    if !expanded_prefixes.is_empty() {
                                        for prefix in expanded_prefixes {
                                            if let Some(bucket) = &app.s3_state.current_bucket.clone() {
                                                let _ = app.load_prefix_preview(bucket.clone(), prefix).await;
                                            }
                                        }
                                    } else {
                                        let _ = app.load_s3_objects().await;
                                    }
                                }
                            }
                            app.s3_state.buckets.loading = false;
                        }

                        // Execute Insights query if loading was triggered
                        if app.current_service == Service::CloudWatchInsights && app.log_groups_state.loading && !prev_loading {
                            app.log_groups_state.loading_message = "Executing query...".to_string();
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            if let Err(e) = app.execute_insights_query().await {
                                app.error_message = Some(format!("Query failed: {:#}", e));
                                app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                            }
                            app.log_groups_state.loading = false;
                            app.log_groups_state.loading_message.clear();
                        }

                        if prev_view_mode != app.view_mode {
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                        }

                        // Handle profile change - reconnect with new credentials
                        if is_profile_select {
                            let profile_name = app.profile.clone();

                            // Check if we have a region
                            if app.region.is_empty() {
                                // No region - open region picker
                                app.measure_region_latencies();
                                app.mode = rusticity_term::keymap::Mode::RegionPicker;
                            } else {
                                // Have region - reconnect
                                app.log_groups_state.loading = true;
                                app.log_groups_state.loading_message = "Connecting with new profile...".to_string();
                                terminal.draw(|f| rusticity_term::ui::render(f, &app))?;

                                let new_app_result = App::new(
                                    Some(profile_name.clone()),
                                    Some(app.region.clone())
                                ).await;

                                match new_app_result {
                                    Ok(mut new_app) => {
                                        new_app.profile = profile_name;
                                        app = new_app;
                                    },
                                    Err(e) => {
                                        app.error_message = Some(format!("Failed to connect: {:#}", e));
                                        app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                    }
                                }
                                app.log_groups_state.loading = false;
                                app.log_groups_state.loading_message.clear();
                            }
                        }

                        // Handle region change - reconnect to get identity and reload content
                        if is_region_select {
                            app.log_groups_state.loading = true;
                            app.log_groups_state.loading_message = "Connecting with new region...".to_string();
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;

                            let profile_name = app.profile.clone();
                            let region_name = app.region.clone();
                            let new_app_result = App::new(
                                Some(profile_name.clone()),
                                Some(region_name.clone())
                            ).await;

                            match new_app_result {
                                Ok(mut new_app) => {
                                    new_app.profile = profile_name;
                                    app = new_app;
                                },
                                Err(e) => {
                                    app.error_message = Some(format!("Failed to connect: {:#}", e));
                                    app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                            app.log_groups_state.loading = false;
                            app.log_groups_state.loading_message.clear();
                        }

                        if should_retry {
                            app.log_groups_state.loading_message = "Reconnecting to AWS...".to_string();
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;

                            let new_app_result = App::new(
                                Some(app.profile.clone()),
                                Some(app.config.region.clone())
                            ).await;

                            match new_app_result {
                                Ok(new_app) => {
                                    app = new_app;
                                    app.log_groups_state.loading = true;
                                    app.log_groups_state.loading_message = "Loading log groups...".to_string();
                                    terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                    if let Err(e) = app.load_log_groups().await {
                                        app.error_message = Some(format!("{:#}", e));
                                        app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                    }
                                    app.log_groups_state.loading = false;
                                    app.log_groups_state.loading_message.clear();
                                    app.log_groups_state.loading_message.clear();
                                },
                                Err(e) => {
                                    app.error_message = Some(format!("{:#}", e));
                                    app.error_scroll = 0;
                                app.mode = rusticity_term::keymap::Mode::ErrorModal;
                                }
                            }
                        }

                        if should_refresh {
                            match app.view_mode {
                                ViewMode::List => {
                                    app.log_groups_state.loading_message = "Refreshing log groups...".to_string();
                                    terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                    let _ = app.load_log_groups().await;
                                    app.log_groups_state.loading = false;
                                    app.log_groups_state.loading_message.clear();
                                },
                                ViewMode::Detail => {
                                    app.log_groups_state.loading_message = "Refreshing log streams...".to_string();
                                    terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                    let _ = app.load_log_streams().await;
                                    app.log_groups_state.loading = false;
                                    app.log_groups_state.loading_message.clear();
                                },
                                ViewMode::Events => {
                                    app.log_groups_state.loading_message = "Refreshing log events...".to_string();
                                    terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                    let _ = app.load_log_events().await;
                                    app.log_groups_state.loading = false;
                                    app.log_groups_state.loading_message.clear();
                                },
                                ViewMode::InsightsResults => {
                                    // No refresh action for insights results
                                },
                                ViewMode::PolicyView => {
                                    // No refresh action for policy view
                                },
                            }
                        }

                        // Load more events if triggered by scrolling up
                        if !prev_loading && app.log_groups_state.loading && app.view_mode == ViewMode::Events {
                            app.log_groups_state.loading_message = "Loading more events...".to_string();
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                            let _ = app.load_log_events().await;
                            app.log_groups_state.loading = false;
                                    app.log_groups_state.loading_message.clear();
                        }

                        // Load data when selecting item in content area
                        if should_load && app.current_service == Service::CloudWatchLogGroups {
                            if app.view_mode == ViewMode::Detail && prev_view_mode == ViewMode::List {
                                app.log_groups_state.loading = true;
                                app.log_groups_state.loading_message = "Loading log streams...".to_string();
                                terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                let _ = app.load_log_streams().await;
                                app.log_groups_state.loading = false;
                                app.log_groups_state.loading_message.clear();
                            } else if app.view_mode == ViewMode::Events && prev_view_mode == ViewMode::Detail {
                                app.log_groups_state.loading = true;
                                app.log_groups_state.loading_message = "Loading log events...".to_string();
                                terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                let _ = app.load_log_events().await;
                                app.log_groups_state.loading = false;
                                app.log_groups_state.loading_message.clear();
                            } else if app.view_mode == ViewMode::List && app.log_groups_state.log_groups.items.is_empty() {
                                app.log_groups_state.loading = true;
                                app.log_groups_state.loading_message = "Loading log groups...".to_string();
                                terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                                let _ = app.load_log_groups().await;
                                app.log_groups_state.loading = false;
                                app.log_groups_state.loading_message.clear();
                            }
                        }

                        terminal.draw(|f| rusticity_term::ui::render(f, &app))?;

                        // Capture snapshot if requested
                        if app.snapshot_requested {
                            app.snapshot_requested = false;

                            // Capture the rendered content
                            let mut snapshot = String::new();
                            terminal.draw(|f| {
                                rusticity_term::ui::render(f, &app);

                                // Extract text from the frame's buffer
                                let size = f.area();
                                let width = size.width as usize;
                                let height = size.height as usize;

                                // Clone buffer content to avoid borrow issues
                                let content: Vec<_> = f.buffer_mut().content().to_vec();

                                let mut lines = Vec::new();
                                for row in 0..height {
                                    let start = row * width;
                                    let end = start + width;
                                    if end <= content.len() {
                                        let line: String = content[start..end]
                                            .iter()
                                            .map(|cell| cell.symbol())
                                            .collect();
                                        lines.push(line.trim_end().to_string());
                                    }
                                }
                                snapshot = lines.join("\n");
                            })?;

                            match arboard::Clipboard::new() {
                                Ok(mut clipboard) => {
                                    match clipboard.set_text(snapshot) {
                                        Ok(_) => {
                                            app.error_message = Some("Snapshot copied to clipboard".to_string());
                                        }
                                        Err(e) => {
                                            app.error_message = Some(format!("Failed to copy: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    app.error_message = Some(format!("Failed to access clipboard: {}", e));
                                }
                            }
                            terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                        }
                        }
                    }
                    Some(Event::Resize(_, _)) => {
                        terminal.draw(|f| rusticity_term::ui::render(f, &app))?;
                    }
                    _ => {}
                }
            }
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
