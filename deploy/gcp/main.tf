# Google Storage Bucket for the function
resource "google_storage_bucket" "function_bucket" {
  name                        = "${var.project_id}-function-storage"
  location                    = var.region
  uniform_bucket_level_access = true
  force_destroy               = true
}

# Persistent Disk for dedicated volume
resource "google_compute_disk" "function_disk" {
  name   = "${var.function_name}-disk"
  type   = "pd-standard"
  zone   = "${var.region}-a"
  size   = var.volume_size_gb
  labels = var.labels
}

# Service Account for the function (if not provided)
resource "google_service_account" "function_sa" {
  count        = var.service_account_email == null ? 1 : 0
  account_id   = "${var.function_name}-sa"
  display_name = "Service Account for ${var.function_name}"
}

locals {
  service_account_email = var.service_account_email != null ? var.service_account_email : google_service_account.function_sa[0].email
}

# IAM binding for the HTTP invocation
resource "google_cloud_run_service_iam_member" "invoker" {
  location = google_cloudfunctions2_function.function.location
  service  = google_cloudfunctions2_function.function.name
  role     = "roles/run.invoker"
  member   = "allUsers" # This allows public access - change to specific users/groups for restricted access
}

# Cloud Function V2 with container image
resource "google_cloudfunctions2_function" "function" {
  name        = var.function_name
  location    = var.region
  description = var.description

  build_config {
    runtime     = "managed"
    entry_point = "handler" # Default entry point, adjust as needed

    source {
      storage_source {
        bucket = google_storage_bucket.function_bucket.name
        object = "source.zip" # This is just a placeholder, as we're using a container image
      }
    }

    docker_repository = "projects/${var.project_id}/repos/${var.function_name}-repo"
    docker_image      = var.container_image
  }

  service_config {
    max_instance_count    = var.max_instance_count
    min_instance_count    = var.min_instance_count
    available_memory      = "${var.available_memory_mb}M"
    timeout_seconds       = var.timeout_seconds
    service_account_email = local.service_account_email

    environment_variables = var.environment_variables

    ingress_settings = var.ingress_settings

    dynamic "vpc_connector" {
      for_each = var.vpc_connector != null ? [1] : []
      content {
        name            = var.vpc_connector
        egress_settings = var.vpc_connector_egress_settings
      }
    }

    # Mount the persistent disk as a volume
    secret_volumes {
      mount_path = var.mount_path
      project_id = var.project_id
      secret     = google_compute_disk.function_disk.name
      versions {
        version = "latest"
        path    = "/"
      }
    }
  }

  # HTTP Trigger
  event_trigger {
    trigger_region        = var.region
    event_type            = "google.cloud.audit.log.v1.written"
    retry_policy          = "RETRY_POLICY_DO_NOT_RETRY"
    service_account_email = local.service_account_email
  }

  labels = var.labels
}
