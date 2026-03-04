use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::enqueue::text_to_image::enqueue_text_to_image_command::EnqueueTextToImageRequest;
use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::events::generation_events::common::{GenerationAction, GenerationModel, GenerationServiceProvider};
use crate::core::events::generation_events::generation_complete_event::GenerationCompleteEvent;
use crate::core::events::generation_events::generation_failed_event::GenerationFailedEvent;
use crate::core::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::task_database::TaskDatabase;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use enums::tauri::tasks::task_status::TaskStatus;
use enums::tauri::tasks::task_type::TaskType;
use idempotency::uuid::generate_random_uuid;
use log::{error, info};
use sqlite_tasks::queries::get_task_by_provider_and_provider_job_id::{get_task_by_provider_and_provider_job_id, GetTaskByProviderAndProviderJobIdArgs};
use sqlite_tasks::queries::update_successful_task_status_with_metadata::{update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs};
use sqlite_tasks::queries::update_task_status::{update_task_status, UpdateTaskArgs};
use std::io::Write;
use tauri::AppHandle;

pub async fn handle_n8n(
  app: &AppHandle,
  request: &EnqueueTextToImageRequest,
  app_prefs: &AppPreferencesManager,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
) -> Result<TaskEnqueueSuccess, GenerateError> {

  let webhook_url = app_prefs
      .get_clone()
      .map_err(|e| GenerateError::AnyhowError(anyhow::anyhow!("Failed to read preferences: {}", e)))?
      .n8n_webhook_url
      .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("N8n webhook URL not configured. Set it in Settings.")))?;

  let prompt = request.prompt
      .as_deref()
      .map(|p| p.trim().to_string())
      .unwrap_or_default();

  let job_id = generate_random_uuid();

  // Clone what we need for the spawned task
  let app_handle = app.clone();
  let app_data_root = app_data_root.clone();
  let task_database = task_database.clone();
  let job_id_clone = job_id.clone();

  tokio::spawn(async move {
    let result = process_n8n_request(
      &app_handle,
      &webhook_url,
      &prompt,
      &job_id_clone,
      &app_data_root,
      &task_database,
    ).await;

    if let Err(err) = result {
      error!("N8n webhook generation failed: {:?}", err);

      let event = GenerationFailedEvent {
        action: GenerationAction::GenerateImage,
        service: GenerationServiceProvider::N8n,
        model: Some(GenerationModel::N8nWebhook),
        reason: Some(format!("{}", err)),
      };
      event.send_infallible(&app_handle);

      // Update task as failed
      if let Ok(Some(task)) = get_task_by_provider_and_provider_job_id(GetTaskByProviderAndProviderJobIdArgs {
        db: task_database.get_connection(),
        provider: GenerationProvider::N8n,
        provider_job_id: &job_id_clone,
      }).await {
        let _ = update_task_status(UpdateTaskArgs {
          db: task_database.get_connection(),
          task_id: &task.id,
          status: TaskStatus::Failed,
        }).await;
      }
    }
  });

  Ok(TaskEnqueueSuccess {
    provider: GenerationProvider::N8n,
    model: Some(GenerationModel::N8nWebhook),
    provider_job_id: Some(job_id),
    task_type: TaskType::ImageGeneration,
  })
}

async fn process_n8n_request(
  app_handle: &AppHandle,
  webhook_url: &str,
  prompt: &str,
  job_id: &str,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
) -> Result<(), anyhow::Error> {

  info!("Sending prompt to n8n webhook: {}", prompt);

  let client = reqwest::Client::new();
  let payload = serde_json::json!({
    "img_prompt": prompt,
    "webhookUrl": webhook_url,
    "executionMode": "production",
  });

  let response = client
      .post(webhook_url)
      .json(&payload)
      .timeout(std::time::Duration::from_secs(120))
      .send()
      .await?;

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    return Err(anyhow::anyhow!("N8n webhook returned {}: {}", status, body));
  }

  let image_bytes = response.bytes().await?;

  info!("Received {} bytes from n8n webhook", image_bytes.len());

  // Save to temp file
  let mut temp_file = app_data_root.temp_dir().new_named_temp_file_with_extension("png")?;
  temp_file.write_all(&image_bytes)?;
  let temp_path = temp_file.path().to_string_lossy().to_string();

  info!("N8n image saved to: {}", temp_path);

  // Update task in database as successful
  let task = get_task_by_provider_and_provider_job_id(GetTaskByProviderAndProviderJobIdArgs {
    db: task_database.get_connection(),
    provider: GenerationProvider::N8n,
    provider_job_id: job_id,
  }).await?;

  if let Some(task) = task {
    let _ = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs {
      db: task_database.get_connection(),
      task_id: &task.id,
      maybe_batch_token: None,
      maybe_primary_media_file_token: None,
      maybe_primary_media_file_class: Some(TaskMediaFileClass::Image),
      maybe_primary_media_file_thumbnail_url_template: None,
      maybe_primary_media_file_cdn_url: None,
    }).await;
  }

  let event = GenerationCompleteEvent {
    action: Some(GenerationAction::GenerateImage),
    service: GenerationServiceProvider::N8n,
    model: Some(GenerationModel::N8nWebhook),
  };
  event.send_infallible(app_handle);

  info!("N8n image generation complete");

  Ok(())
}
