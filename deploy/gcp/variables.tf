variable "project_id" {
  description = "The Google Cloud project ID"
  type        = string
}

variable "region" {
  description = "The GCP region to deploy resources"
  type        = string
  default     = "us-central1"
}

variable "service_name" {
  description = "The name of the Cloud Run service"
  type        = string
  default     = "my-cloud-run-service"
}

variable "container_image" {
  description = "The Docker container image to deploy (e.g., gcr.io/project-id/image:tag)"
  type        = string
}

variable "cpu" {
  description = "CPU allocation for the Cloud Run service"
  type        = string
  default     = "1"
}

variable "memory" {
  description = "Memory allocation for the Cloud Run service"
  type        = string
  default     = "512Mi"
}

variable "volume_mount_path" {
  description = "The path where the volume should be mounted in the container"
  type        = string
  default     = "/mnt/data"
}

variable "volume_size" {
  description = "Size of the persistent volume"
  type        = string
  default     = "10Gi"
}

variable "env_vars" {
  description = "Environment variables for the Cloud Run service"
  type        = map(string)
  default     = {}
}
