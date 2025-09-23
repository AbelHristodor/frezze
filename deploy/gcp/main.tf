# Create a Cloud Run service with a Docker image
resource "google_cloud_run_v2_service" "service" {
  name     = var.service_name
  location = var.region


  scaling {
    max_instance_count = 1
  }

  template {
    containers {
      image = var.container_image

      # Resource limits
      resources {
        limits = {
          cpu    = var.cpu
          memory = var.memory
        }
      }

      # Volume mount configuration
      volume_mounts {
        name       = "service-data"
        mount_path = var.volume_mount_path
      }

      # Environment variables (if needed)
      dynamic "env" {
        for_each = var.env_vars
        content {
          name  = env.key
          value = env.value
        }
      }
    }

    # Volume configuration
    volumes {
      name = "service-data"

    }

  }


  # Automatically determine traffic behavior
  traffic {
    percent = 100
  }

  # Depends on the service account
  depends_on = [
    google_service_account.cloud_run_sa
  ]
}

# Create a Kubernetes Persistent Volume Claim for the volume
resource "google_persistent_volume_claim" "service_data" {
  metadata {
    name = "${var.service_name}-data"
  }
  spec {
    access_modes = ["ReadWriteOnce"]
    resources {
      requests = {
        storage = var.volume_size
      }
    }
  }
}

# Create a service account for the Cloud Run service
resource "google_service_account" "cloud_run_sa" {
  account_id   = "${var.service_name}-sa"
  display_name = "Service Account for ${var.service_name} Cloud Run Service"
}

# Grant required permissions to the service account
resource "google_project_iam_member" "cloud_run_sa_permissions" {
  for_each = toset([
    "roles/logging.logWriter",
    "roles/monitoring.metricWriter",
    "roles/cloudtrace.agent",
  ])

  project = var.project_id
  role    = each.key
  member  = "serviceAccount:${google_service_account.cloud_run_sa.email}"
}

# Allow unauthenticated invocations (HTTP trigger)
resource "google_cloud_run_service_iam_member" "public_access" {
  location = google_cloud_run_service.service.location
  service  = google_cloud_run_service.service.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}

